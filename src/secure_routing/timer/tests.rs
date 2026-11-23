use super::*;
use std::{cell::Cell, rc::Rc};

const GROUP: Ipv4Addr = Ipv4Addr::new(239, 1, 2, 3);
const TARGET: SocketAddrV4 = SocketAddrV4::new(GROUP, 3671);
const LOCAL: SocketAddrV4 = SocketAddrV4::new(Ipv4Addr::LOCALHOST, 42000);
const REMOTE: SocketAddrV4 = SocketAddrV4::new(Ipv4Addr::LOCALHOST, 42001);
const SERIAL: [u8; 6] = [0, 0, 0, 0, 0, 42];
const OTHER: [u8; 6] = [0, 0, 0, 0, 0, 7];

#[derive(Clone, Default)]
struct Store {
    ceiling: Rc<Cell<Option<u64>>>,
    unavailable: Rc<Cell<bool>>,
}

impl TimerStore for Store {
    fn origin(&mut self) -> io::Result<TimerOrigin> {
        if self.unavailable.get() {
            return Err(io::Error::other("unavailable"));
        }
        Ok(self
            .ceiling
            .get()
            .map_or(TimerOrigin::NewKey, TimerOrigin::ReservedThrough))
    }

    fn reserve_through(&mut self, upper: u64) -> io::Result<()> {
        if self.unavailable.get() {
            return Err(io::Error::other("unavailable"));
        }
        assert!(self.ceiling.get().is_none_or(|last| upper > last));
        self.ceiling.set(Some(upper));
        Ok(())
    }
}

fn config() -> TimerConfig {
    TimerConfig {
        interface_index: 7,
        multicast_group: GROUP,
        local_source: LOCAL,
        serial: SERIAL,
        latency_ms: 100,
        sync_fraction: 26,
    }
}

fn key() -> [u8; 16] {
    std::array::from_fn(|i| i as u8)
}

fn start(now: Instant, store: Store) -> GroupClock<Store> {
    GroupClock::new(config(), key(), store, now, TimerStart::Immediate).unwrap()
}

fn notification(time: u64, serial: [u8; 6], tag: u16) -> [u8; 36] {
    let mut bytes = [0; 36];
    bytes[..6].copy_from_slice(&[6, 16, 9, 0x55, 0, 36]);
    bytes[6..12].copy_from_slice(&time.to_be_bytes()[2..]);
    bytes[12..18].copy_from_slice(&serial);
    bytes[18..20].copy_from_slice(&tag.to_be_bytes());
    let mac = ip_secure_crypto::timer_mac(&key(), bytes[..20].try_into().unwrap());
    bytes[20..].copy_from_slice(&mac);
    bytes
}

fn wrapper(time: u64, tag: u16) -> Vec<u8> {
    ip_secure_crypto::seal_with_tag(
        &key(),
        0,
        time,
        OTHER,
        tag,
        &[6, 16, 5, 0x31, 0, 10, 4, 0, 0, 3],
    )
    .unwrap()
}

fn see<S: TimerStore>(
    clock: &mut GroupClock<S>,
    now: Instant,
    bytes: &[u8],
) -> io::Result<Option<u64>> {
    clock.observe(now, 7, TARGET, REMOTE, bytes)
}

fn emit(clock: &mut GroupClock<Store>, now: Instant) -> [u8; 36] {
    let mut out = [0; 36];
    assert!(
        clock
            .poll(now, |target, bytes| {
                assert_eq!(target, TARGET);
                out = *bytes;
                Ok(())
            })
            .unwrap()
    );
    out
}

#[test]
fn acquisition_requires_the_wait_or_authenticated_echo_and_never_accepts_own_notification() {
    let epoch = Instant::now();
    let store = Store::default();
    let mut clock = start(epoch, store.clone());
    assert_eq!(clock.sample(epoch).unwrap(), None);
    assert!(clock.receiver(epoch, OTHER).is_err());
    assert!(clock.wrapper_sent(epoch).is_err());
    assert!(
        clock
            .poll(epoch, |_, _| Err(io::Error::other("socket unavailable")))
            .is_err()
    );
    assert_eq!(clock.deadline(), epoch);
    let challenge = emit(&mut clock, epoch);
    let tag = u16::from_be_bytes(challenge[18..20].try_into().unwrap());
    assert_eq!(
        clock.observe(epoch, 7, TARGET, LOCAL, &challenge).unwrap(),
        None
    );
    assert_eq!(see(&mut clock, epoch, &challenge).unwrap(), None);
    assert_eq!(clock.sample(epoch).unwrap(), None);
    // An unrelated authentic notify can move tentative time but cannot complete this challenge.
    see(&mut clock, epoch, &notification(900, OTHER, tag)).unwrap();
    see(
        &mut clock,
        epoch,
        &notification(901, SERIAL, tag.wrapping_add(1)),
    )
    .unwrap();
    assert_eq!(clock.sample(epoch).unwrap(), None);
    assert_eq!(see(&mut clock, epoch, &wrapper(902, tag)).unwrap(), None);
    see(&mut clock, epoch, &notification(903, SERIAL, tag)).unwrap();
    assert_eq!(clock.sample(epoch).unwrap(), Some(903));
    let mut peer = clock.receiver(epoch, OTHER).unwrap();
    let bytes = wrapper(904, tag);
    let time = see(&mut clock, epoch, &bytes).unwrap();
    assert!(matches!(
        peer.receive(7, TARGET, time, &bytes).unwrap().event(),
        crate::routing::RoutingEvent::Lost { messages: 3, .. }
    ));
    assert!(peer.receive(7, TARGET, time, &bytes).is_err());
    assert!(clock.sample(epoch - delay(1)).is_err());
}

#[test]
fn authenticated_events_follow_keeper_follower_and_update_ranges() {
    let epoch = Instant::now();
    let mut clock = start(epoch, Store::default());
    emit(&mut clock, epoch);
    let at = epoch + delay(420); // 100 + 12 * 10 + 2 * 100.
    assert_eq!(clock.sample(at - delay(1)).unwrap(), None);
    assert_eq!(clock.sample(at).unwrap(), Some(420));
    assert!(clock.keeper);
    // Exactly latencyTolerance behind triggers one scheduled update.
    assert_eq!(see(&mut clock, at, &wrapper(320, 23)).unwrap(), None);
    let due = clock.deadline();
    assert!((at + delay(100)..=at + delay(110)).contains(&due));
    see(&mut clock, at, &notification(20, SERIAL, 90)).unwrap();
    assert_eq!(clock.update, Some((OTHER, 23)));
    assert_eq!(see(&mut clock, at, &wrapper(429, 24)).unwrap(), Some(429));
    assert!(clock.keeper);
    clock.wrapper_sent(at).unwrap();
    assert_eq!(clock.deadline(), due);
    let update = emit(&mut clock, due);
    assert_eq!(&update[12..18], &OTHER);
    assert_eq!(&update[18..20], &23u16.to_be_bytes());
    assert!(clock.keeper);
    assert!((due + delay(10_000)..=due + delay(10_030)).contains(&clock.deadline()));
    let local = clock.sample(due).unwrap().unwrap();
    see(&mut clock, due, &notification(local - 100, OTHER, 5)).unwrap();
    let pending = clock.deadline();
    // Older than sync tolerance but within latency changes neither timer nor schedule.
    see(&mut clock, due, &notification(local - 10, OTHER, 6)).unwrap();
    assert_eq!(
        see(&mut clock, due, &wrapper(local - 99, 8)).unwrap(),
        Some(local)
    );
    assert_eq!(clock.deadline(), pending);
    see(&mut clock, due, &notification(local - 9, OTHER, 7)).unwrap();
    assert!(clock.update.is_none());
    assert!(!clock.keeper);
    assert!((due + delay(10_040)..=due + delay(10_140)).contains(&clock.deadline()));
    see(&mut clock, due, &notification(local - 101, OTHER, 8)).unwrap();
    assert!((due + delay(120)..=due + delay(220)).contains(&clock.deadline()));
    see(&mut clock, due, &notification(local + 10, OTHER, 9)).unwrap();
    assert!(clock.update.is_none());
    assert_eq!(clock.sample(due).unwrap(), Some(local + 10));
}

#[test]
fn xknx_timer_authentication_membership_and_persistent_fences() {
    // XKNX 4.1.1 SecureSequenceTimer.send_timer_notify, key 00..0f,
    // timer 123456, serial 00fa12345678, tag 1234.
    let frame = notification(123456, [0, 0xfa, 0x12, 0x34, 0x56, 0x78], 0x1234);
    assert_eq!(
        &frame[20..],
        &[
            0xdf, 0xa7, 0x3d, 0x34, 0x5f, 0xd0, 0xc0, 0x4d, 0x8f, 0xe6, 0xbe, 0x50, 0x1d, 0x62,
            0x34, 0xab
        ]
    );
    let epoch = Instant::now();
    let store = Store::default();
    let mut clock = start(epoch, store.clone());
    for i in [0, 3, 5, 6, 12, 18, 20, 35] {
        let mut bad = frame;
        bad[i] ^= 1;
        assert!(see(&mut clock, epoch, &bad).is_err());
    }
    assert!(see(&mut clock, epoch, &frame[..35]).is_err());
    let mut long = frame.to_vec();
    long.push(0);
    assert!(see(&mut clock, epoch, &long).is_err());
    for (index, target) in [
        (1, TARGET),
        (7, SocketAddrV4::new(GROUP, 1)),
        (7, SocketAddrV4::new(Ipv4Addr::LOCALHOST, 3671)),
    ] {
        assert!(clock.observe(epoch, index, target, REMOTE, &frame).is_err());
    }
    let unicast = ip_secure_crypto::seal_with_tag(
        &key(),
        1,
        123456,
        OTHER,
        1,
        &[6, 16, 5, 0x31, 0, 10, 4, 0, 0, 3],
    )
    .unwrap();
    assert!(see(&mut clock, epoch, &unicast).is_err());
    assert_eq!(clock.deadline(), epoch);
    assert_eq!(store.ceiling.get(), Some(60000));
    see(&mut clock, epoch, &frame).unwrap();
    assert_eq!(store.ceiling.get(), Some(183456));
    store.unavailable.set(true);
    assert!(see(&mut clock, epoch, &notification(200000, OTHER, 1)).is_err());
    assert!(clock.sample(epoch).is_err());
    store.unavailable.set(false);
    assert!(clock.sample(epoch).is_err()); // uncertain store commit requires reopening
    drop(clock);
    let mut reboot = start(epoch, store.clone());
    assert_eq!(reboot.base, 183457);
    assert_eq!(see(&mut reboot, epoch, &frame).unwrap(), None);
    assert_eq!(reboot.base, 183457);
    store.ceiling.set(Some(MAX_TIMER));
    assert!(GroupClock::new(config(), key(), store, epoch, TimerStart::Immediate).is_err());
}

#[test]
fn first_boot_delay_and_restart_on_new_domain() {
    let epoch = Instant::now();
    let mut cfg = config();
    let new = GroupClock::new(cfg, key(), Store::default(), epoch, TimerStart::PowerUp).unwrap();
    assert!((epoch..=epoch + delay(10_000)).contains(&new.deadline()));
    let unavailable = Store::default();
    unavailable.unavailable.set(true);
    assert!(GroupClock::new(cfg, key(), unavailable, epoch, TimerStart::PowerUp).is_err());
    cfg.interface_index = 0;
    assert!(GroupClock::new(cfg, key(), Store::default(), epoch, TimerStart::Immediate).is_err());
}

#[test]
fn reopened_file_store_keeps_distinct_key_epochs_and_restored_time() {
    use std::{
        fs::{self, File},
        io::Write,
        path::PathBuf,
    };
    struct FileStore(PathBuf);
    impl TimerStore for FileStore {
        fn origin(&mut self) -> io::Result<TimerOrigin> {
            // This fixture provisions each key record explicitly before the first boot.
            let value = fs::read_to_string(&self.0)?;
            if value == "new" {
                return Ok(TimerOrigin::NewKey);
            }
            value
                .parse()
                .map(TimerOrigin::ReservedThrough)
                .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "damaged timer record"))
        }
        fn reserve_through(&mut self, upper: u64) -> io::Result<()> {
            let tmp = self.0.with_extension("tmp");
            let mut file = File::create(&tmp)?;
            write!(file, "{upper}")?;
            file.sync_all()?;
            fs::rename(tmp, &self.0)?;
            #[cfg(unix)]
            File::open(self.0.parent().unwrap())?.sync_all()?;
            Ok(())
        }
    }
    let root = std::env::temp_dir().join(format!(
        "knx-timer-{}-{}",
        std::process::id(),
        getrandom::u64().unwrap()
    ));
    fs::create_dir(&root).unwrap();
    let a = root.join("a");
    let b = root.join("b");
    fs::write(&a, b"new").unwrap();
    fs::write(&b, b"new").unwrap();
    let at = Instant::now();
    let open = |path: &PathBuf, key, now| {
        GroupClock::new(
            config(),
            key,
            FileStore(path.clone()),
            now,
            TimerStart::Immediate,
        )
    };
    let mut first = open(&a, key(), at).unwrap();
    see(&mut first, at, &notification(1_000_000, OTHER, 7)).unwrap();
    drop(first);
    assert_eq!(open(&b, [7; 16], at).unwrap().base, 0);
    let mut restored = open(&a, key(), at).unwrap();
    assert_eq!(restored.base, 1_060_001);
    assert!(restored.sample(at).unwrap().is_none());
    assert!(
        see(&mut restored, at, &notification(1_000_000, OTHER, 7))
            .unwrap()
            .is_none()
    );
    fs::write(&a, b"broken").unwrap();
    assert!(open(&a, key(), at).is_err());
    fs::remove_file(&a).unwrap();
    assert!(open(&a, key(), at).is_err());
    fs::remove_dir_all(root).unwrap();
}
