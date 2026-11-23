#![cfg(feature = "data-secure")]

use knx::data_secure::{GroupReceiver, ReceiveError, SequenceStore};
use knx::telegram::{GroupData, GroupService};
use std::sync::atomic::{AtomicU64, Ordering};

// XKNX SecureTest.knxkeys and test_data_secure.py (public synthetic test installation).
const KEY: [u8; 16] = [
    0xdf, 0xdf, 0x23, 0xa5, 0x9f, 0xbb, 0x40, 0x40, 0x40, 0x91, 0xd1, 0xc1, 0x62, 0x08, 0x7e, 0x8b,
];
const READ: &str = "2900bce0500104000e03f11000254ae1cb67cd184afe5744";
const WRITE: &str = "29003ce0500104001103f11000254ae1cb67cd98e577b519be47bb";
const RESPONSE: &str = "29003ce0400904001103f110002446cfef4ac085e7092ab062b44d";

fn bytes(s: &str) -> Vec<u8> {
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap())
        .collect()
}
fn receiver(sender: u16) -> GroupReceiver {
    GroupReceiver::new(sender, "0/4/0".parse().unwrap(), KEY)
}
struct Store<'a>(&'a AtomicU64);
impl SequenceStore for Store<'_> {
    type Error = ();
    fn advance(&mut self, _: u16, seq: u64) -> Result<bool, ()> {
        Ok(self.0.fetch_max(seq, Ordering::SeqCst) < seq)
    }
}

#[test]
fn independent_wire_vectors_cover_reads_responses_and_writes() {
    for (sender, frame, expected) in [
        (0x5001, READ, GroupService::Read),
        (
            0x5001,
            WRITE,
            GroupService::Write(GroupData::Bytes(&[255, 0, 5])),
        ),
        (
            0x4009,
            RESPONSE,
            GroupService::Response(GroupData::Bytes(&[116, 41, 41])),
        ),
    ] {
        let high = AtomicU64::new(1);
        let result = receiver(sender)
            .receive(&bytes(frame), &mut Store(&high))
            .unwrap();
        assert_eq!(result.service(), expected);
        assert_eq!(result.sequence, high.load(Ordering::Relaxed));
        assert!(matches!(
            receiver(sender).receive(&bytes(frame), &mut Store(&high)),
            Err(ReceiveError::Replay)
        ));
    }
}

#[test]
fn concurrent_receivers_accept_a_sequence_only_once() {
    let high = AtomicU64::new(0);
    let gate = std::sync::Barrier::new(2);
    std::thread::scope(|scope| {
        let run = || {
            gate.wait();
            receiver(0x5001)
                .receive(&bytes(READ), &mut Store(&high))
                .map(|r| r.sequence)
        };
        let peer = scope.spawn(run);
        let pair = [run(), peer.join().unwrap()];
        assert_eq!(pair.iter().filter(|r| r.is_ok()).count(), 1);
        assert!(pair.contains(&Err(ReceiveError::Replay)));
    });
}

#[test]
fn failures_do_not_deliver_plaintext_or_advance_the_sequence() {
    struct MissingState;
    impl SequenceStore for MissingState {
        type Error = ();
        fn advance(&mut self, _: u16, _: u64) -> Result<bool, ()> {
            Err(())
        }
    }
    let raw = bytes(WRITE);
    assert!(matches!(
        receiver(0x5001).receive(&raw, &mut MissingState),
        Err(ReceiveError::StateUnavailable)
    ));
    for index in [
        0, 1, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25,
        26,
    ] {
        let mut bad = raw.clone();
        bad[index] ^= 1;
        let high = AtomicU64::new(0);
        assert!(
            receiver(0x5001).receive(&bad, &mut Store(&high)).is_err(),
            "{index}"
        );
        assert_eq!(high.load(Ordering::Relaxed), 0);
    }
    let mut escape = raw.clone();
    escape.resize(265, 0);
    escape[8] = 255;
    assert!(matches!(
        receiver(0x5001).receive(&escape, &mut MissingState),
        Err(ReceiveError::Unsupported)
    ));
    let other = GroupReceiver::new(0x5001, "0/4/0".parse().unwrap(), [0; 16]);
    assert!(matches!(
        other.receive(&raw, &mut MissingState),
        Err(ReceiveError::Authentication)
    ));
    for n in 0..raw.len() {
        assert!(matches!(
            receiver(0x5001).receive(&raw[..n], &mut MissingState),
            Err(ReceiveError::Unsupported)
        ));
    }
}

#[test]
fn maximum_sequence_is_accepted_once_and_zero_never_wraps() {
    let binding = GroupReceiver::new(
        0x5001,
        "0/4/0".parse().unwrap(),
        std::array::from_fn(|i| i as u8),
    );
    let last = bytes(
        "29003ce0500104002e03f110ffffffffffff4a09f5975e0909ddc1545bc9961eb17336983d0f789b0ac882caff846540e512a89eef7dadf8",
    );
    let zero = bytes(
        "29003ce0500104002e03f110000000000000ec2c7b3f9fa5d7152e2b8b0f314a0ce79a6e3b98cb2d9d259e710d61ed191bfec10724c91f3d",
    );
    let high = AtomicU64::new(0);
    let got = binding.receive(&last, &mut Store(&high)).unwrap();
    let payload = std::array::from_fn::<_, 32, _>(|i| i as u8);
    assert_eq!(
        got.service(),
        GroupService::Write(GroupData::Bytes(&payload))
    );
    assert_eq!(got.sequence, (1 << 48) - 1);
    assert!(matches!(
        binding.receive(&last, &mut Store(&high)),
        Err(ReceiveError::Replay)
    ));
    assert!(matches!(
        binding.receive(&zero, &mut Store(&high)),
        Err(ReceiveError::Replay)
    ));
}

// A host-supplied durable store for one commissioned sender. Exercise reopening the
// file, including an unavailable baseline, without introducing a database dependency.
#[test]
fn reopening_a_commissioned_store_preserves_the_high_water_mark() {
    use std::{
        fs::{self, File, OpenOptions},
        io::{self, Read, Seek, Write},
    };
    struct Records(File);
    impl SequenceStore for Records {
        type Error = io::Error;
        fn advance(&mut self, sender: u16, seq: u64) -> io::Result<bool> {
            assert_eq!(sender, 0x5001);
            let mut bytes = [0; 8];
            self.0.rewind()?;
            self.0.read_exact(&mut bytes)?;
            if seq <= u64::from_be_bytes(bytes) {
                return Ok(false);
            }
            self.0.rewind()?;
            self.0.write_all(&seq.to_be_bytes())?;
            self.0.sync_all()?;
            Ok(true)
        }
    }
    let path = std::env::temp_dir().join(format!(
        "knx-secure-test-{}-{:?}",
        std::process::id(),
        std::thread::current().id()
    ));
    let mut baseline = File::create_new(&path).unwrap();
    baseline.write_all(&1u64.to_be_bytes()).unwrap();
    drop(baseline);
    let reopen = || {
        Records(
            OpenOptions::new()
                .read(true)
                .write(true)
                .open(&path)
                .unwrap(),
        )
    };
    assert!(
        receiver(0x5001)
            .receive(&bytes(READ), &mut reopen())
            .is_ok()
    );
    assert!(matches!(
        receiver(0x5001).receive(&bytes(READ), &mut reopen()),
        Err(ReceiveError::Replay)
    ));
    File::create(&path).unwrap();
    assert!(matches!(
        receiver(0x5001).receive(&bytes(READ), &mut reopen()),
        Err(ReceiveError::StateUnavailable)
    ));
    fs::remove_file(path).unwrap();
}

use knx::data_secure::{GroupSender, MAX_SEQUENCE, OutgoingSequenceStore, SendError};

struct Reservation(Result<Option<u64>, ()>);
impl OutgoingSequenceStore for Reservation {
    type Error = ();
    fn reserve(&mut self, _: u16) -> Result<Option<u64>, ()> {
        std::mem::replace(&mut self.0, Err(()))
    }
}
#[test]
fn sealed_services_match_xknx_wire_vectors() {
    for (source, frame, service) in [
        (0x5001, READ, GroupService::Read),
        (
            0x5001,
            WRITE,
            GroupService::Write(GroupData::Bytes(&[255, 0, 5])),
        ),
        (
            0x4009,
            RESPONSE,
            GroupService::Response(GroupData::Bytes(&[116, 41, 41])),
        ),
    ] {
        let mut expected = bytes(frame);
        expected[0] = 0x11;
        let mut n = [0; 8];
        n[2..].copy_from_slice(&expected[12..18]);
        let sender = GroupSender::new(source, "0/4/0".parse().unwrap(), KEY);
        let request = sender
            .request(service, &mut Reservation(Ok(Some(u64::from_be_bytes(n)))))
            .unwrap();
        assert_eq!(request.source(), source);
        assert_eq!(request.as_bytes(), expected);
    }
}
#[test]
fn outgoing_bounds_and_fail_closed_storage() {
    let sender = GroupSender::new(0x5001, "0/4/0".parse().unwrap(), KEY);
    let mut state = Reservation(Ok(Some(MAX_SEQUENCE)));
    for data in [
        GroupData::Inline(64),
        GroupData::Bytes(&[]),
        GroupData::Bytes(&[0; 241]),
    ] {
        assert!(matches!(
            sender.request(GroupService::Write(data), &mut state),
            Err(SendError::InvalidPayload)
        ));
        assert_eq!(state.0, Ok(Some(MAX_SEQUENCE)));
    }
    let req = sender
        .request(
            GroupService::Write(GroupData::Bytes(&[17; 240])),
            &mut state,
        )
        .unwrap();
    assert_eq!(req.as_bytes().len(), 264);
    assert_eq!(&req.as_bytes()[12..18], &[255; 6]);
    for (result, error) in [
        (Ok(None), SendError::SequenceExhausted),
        (Err(()), SendError::StateUnavailable),
        (Ok(Some(0)), SendError::StateUnavailable),
        (Ok(Some(MAX_SEQUENCE + 1)), SendError::StateUnavailable),
    ] {
        assert!(
            matches!(sender.request(GroupService::Read,&mut Reservation(result)),Err(e) if e==error)
        );
    }
}

#[test]
fn secure_tunnel_binds_the_source_and_retries_the_same_datagram_once() {
    use knx::tunnel::{Event, Tunnel, TunnelError};
    use std::net::{Ipv4Addr, SocketAddrV4};
    let peer = SocketAddrV4::new(Ipv4Addr::LOCALHOST, 3671);
    let mut tunnel = Tunnel::accept(
        peer,
        peer,
        &[
            6, 16, 2, 6, 0, 20, 21, 0, 8, 1, 127, 0, 0, 1, 14, 87, 4, 4, 17, 42,
        ],
    )
    .unwrap();
    let seal = |source| {
        GroupSender::new(source, "0/4/0".parse().unwrap(), KEY)
            .request(
                GroupService::Write(GroupData::Bytes(&[82; 240])),
                &mut Reservation(Ok(Some(8))),
            )
            .unwrap()
    };
    assert_eq!(
        tunnel.secure_request(&seal(0x5001)),
        Err(TunnelError::Unexpected)
    );
    let request = seal(tunnel.assigned_address());
    let datagram = tunnel.secure_request(&request).unwrap().to_vec();
    assert_eq!(&datagram[10..], request.as_bytes());
    assert_eq!(tunnel.secure_request(&request), Err(TunnelError::Busy));
    assert_eq!(tunnel.ack_timeout(), Ok(Event::Retry(&datagram)));
    assert_eq!(tunnel.ack_timeout(), Ok(Event::Indeterminate));
}

// Use a persisted, precommissioned counter and a separate lock inode so two host
// instances serialize reservations even as they replace the counter atomically.
#[cfg(unix)]
#[test]
fn durable_reservations_survive_restart_dropped_sends_and_key_rotation() {
    use std::{
        fs::{self, File},
        io::{self, Write},
        path::Path,
    };
    struct Records<'a>(&'a Path);
    impl OutgoingSequenceStore for Records<'_> {
        type Error = io::Error;
        fn reserve(&mut self, sender: u16) -> io::Result<Option<u64>> {
            assert_eq!(sender, 0x5001);
            let lock = File::open(self.0.join("lock"))?;
            lock.lock()?;
            let counter = self.0.join("counter");
            let raw: [u8; 8] = fs::read(&counter)?
                .try_into()
                .map_err(|_| io::ErrorKind::InvalidData)?;
            let last = u64::from_be_bytes(raw);
            if last == MAX_SEQUENCE {
                return Ok(None);
            }
            if last > MAX_SEQUENCE {
                return Err(io::ErrorKind::InvalidData.into());
            }
            let next = last + 1;
            let pending = self.0.join("pending");
            let mut file = File::create(&pending)?;
            file.write_all(&next.to_be_bytes())?;
            file.sync_all()?;
            fs::rename(pending, counter)?;
            File::open(self.0)?.sync_all()?;
            Ok(Some(next))
        }
    }
    let dir = std::env::temp_dir().join(format!(
        "knx-secure-reserve-{}-{:?}",
        std::process::id(),
        std::thread::current().id()
    ));
    fs::create_dir(&dir).unwrap();
    File::create_new(dir.join("lock")).unwrap();
    fs::write(dir.join("counter"), (MAX_SEQUENCE - 5).to_be_bytes()).unwrap();
    let reserve = |group: &str, key| {
        GroupSender::new(0x5001, group.parse().unwrap(), key)
            .request(GroupService::Read, &mut Records(&dir))
    };
    let seq = |req: knx::data_secure::SecureRequest| {
        let mut n = [0u8; 8];
        n[2..].copy_from_slice(&req.as_bytes()[12..18]);
        u64::from_be_bytes(n)
    };
    // Simulate a committed request lost before delivery. Each later call reopens state.
    drop(reserve("0/4/0", KEY).unwrap());
    std::thread::scope(|scope| {
        let p = scope.spawn(|| seq(reserve("0/4/0", KEY).unwrap()));
        let q = seq(reserve("2/1/7", [33; 16]).unwrap());
        let mut assigned = [q, p.join().unwrap()];
        assigned.sort();
        assert_eq!(assigned, [MAX_SEQUENCE - 3, MAX_SEQUENCE - 2]);
    });
    // Restore the old group key while two more sequence numbers are available.
    assert_eq!(seq(reserve("0/4/0", KEY).unwrap()), MAX_SEQUENCE - 1);
    assert_eq!(seq(reserve("0/4/0", KEY).unwrap()), MAX_SEQUENCE);
    assert!(matches!(
        reserve("0/4/0", KEY),
        Err(SendError::SequenceExhausted)
    ));
    fs::remove_file(dir.join("counter")).unwrap();
    assert!(matches!(
        reserve("0/4/0", KEY),
        Err(SendError::StateUnavailable)
    ));
    fs::remove_dir_all(dir).unwrap();
}
