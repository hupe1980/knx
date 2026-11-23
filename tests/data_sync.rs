#![cfg(feature = "data-secure")]
use knx::data_secure::{
    MAX_SEQUENCE, OutgoingSequenceStore, SecureRequest,
    sync::{NextSequences, SyncError, SyncPeer, SyncResponder, SyncStore, SyncUpdate},
};
use std::time::{Duration, Instant};

const LOCAL: u16 = 0x1101;
const PEER: u16 = 0x1102;
fn key() -> [u8; 16] {
    std::array::from_fn(|i| i as u8)
}
struct Store {
    state: NextSequences,
    calls: u32,
    fail: bool,
}
impl Store {
    fn new(local: u64, peer: u64) -> Self {
        Self {
            state: NextSequences { local, peer },
            calls: 0,
            fail: false,
        }
    }
}
impl OutgoingSequenceStore for Store {
    type Error = ();
    fn reserve(&mut self, _: u16) -> Result<Option<u64>, ()> {
        if self.fail {
            return Err(());
        }
        let n = self.state.local;
        if n > MAX_SEQUENCE {
            return Ok(None);
        }
        self.state.local += 1;
        Ok(Some(n))
    }
}
impl SyncStore for Store {
    type Error = ();
    fn reconcile(
        &mut self,
        _: u16,
        _: u16,
        floor: NextSequences,
        _: SyncUpdate,
    ) -> Result<NextSequences, ()> {
        self.calls += 1;
        if self.fail {
            return Err(());
        }
        self.state.local = self.state.local.max(floor.local);
        self.state.peer = self.state.peer.max(floor.peer);
        Ok(self.state)
    }
}
fn ind(request: &SecureRequest) -> Vec<u8> {
    let mut wire = request.as_bytes().to_vec();
    wire[0] = 0x29;
    wire
}

#[test]
fn matches_calimero_request_and_never_reopens_or_lowers_state() {
    // DataSyncPeer.java without arguments generates this fixed normal-mode cEMI indication.
    let hex = "29001060110211011803f11200000000006400000000000036dcbfd1651960eede09";
    let frame = (0..hex.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).unwrap())
        .collect::<Vec<_>>();
    let binding = SyncPeer::new(LOCAL, PEER, key()).unwrap();
    let now = Instant::now();
    let mut state = Store::new(200, 1);
    let mut responder = SyncResponder::new(LOCAL);
    assert!(responder.respond(&binding, &frame, now, &mut state).is_ok());
    assert_eq!(
        state.state,
        NextSequences {
            local: 200,
            peer: 100
        }
    );
    for delta in [0, 999] {
        assert!(matches!(
            responder.respond(
                &binding,
                &frame,
                now + Duration::from_millis(delta),
                &mut state
            ),
            Err(SyncError::RateLimited)
        ));
    }
    state.state.peer = 112;
    assert!(
        responder
            .respond(&binding, &frame, now + Duration::from_secs(1), &mut state)
            .is_ok()
    );
    assert_eq!(state.state.peer, 112);
}

#[test]
fn pending_challenge_binds_source_key_destination_deadline_and_single_commit() {
    let local = SyncPeer::new(LOCAL, PEER, key()).unwrap();
    let peer = SyncPeer::new(PEER, LOCAL, key()).unwrap();
    let mut a = Store::new(11, 2);
    let mut b = Store::new(20, 99);
    let now = Instant::now();
    let (req, mut pending) = local.request(now, &mut a).unwrap();
    let (other, mut stale) = local.request(now, &mut a).unwrap();
    assert_ne!(req.as_bytes(), other.as_bytes());
    let mut responder = SyncResponder::new(PEER);
    let frame = ind(&responder.respond(&peer, &ind(&req), now, &mut b).unwrap());
    assert_eq!(
        stale.complete(&frame, now, &mut a),
        Err(SyncError::Authentication)
    );
    for i in [0, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 17, 18, 24, 30, 33] {
        let mut changed = frame.clone();
        changed[i] ^= if i == 3 { 0x80 } else { 1 };
        assert!(pending.complete(&changed, now, &mut a).is_err(), "{i}");
    }
    assert_eq!(a.calls, 0);
    a.fail = true;
    assert_eq!(
        pending.complete(&frame, now, &mut a),
        Err(SyncError::StateUnavailable)
    );
    a.fail = false;
    pending
        .complete(&frame, now + Duration::from_millis(5999), &mut a)
        .unwrap();
    assert_eq!(
        a.state,
        NextSequences {
            local: 99,
            peer: 20
        }
    );
    assert_eq!(
        pending.complete(&frame, now, &mut a),
        Err(SyncError::Completed)
    );
    assert_eq!(
        stale.complete(&frame, now + Duration::from_secs(6), &mut a),
        Err(SyncError::Expired)
    );
    assert_eq!(
        stale.complete(&frame, now, &mut a),
        Err(SyncError::Completed)
    );
}

#[test]
fn failed_state_wrong_keys_and_unsupported_envelopes_emit_no_reply() {
    let a = SyncPeer::new(LOCAL, PEER, key()).unwrap();
    let b = SyncPeer::new(PEER, LOCAL, key()).unwrap();
    let now = Instant::now();
    let mut store = Store::new(10, 1);
    store.fail = true;
    assert!(matches!(
        a.request(now, &mut store),
        Err(SyncError::StateUnavailable)
    ));
    store.fail = false;
    store.state.local = MAX_SEQUENCE + 1;
    assert!(matches!(
        a.request(now, &mut store),
        Err(SyncError::SequenceExhausted)
    ));
    store.state.local = 20;
    let (req, _) = a.request(now, &mut store).unwrap();
    let frame = ind(&req);
    let mut responder = SyncResponder::new(PEER);
    let wrong = SyncPeer::new(PEER, LOCAL, [8; 16]).unwrap();
    assert!(matches!(
        responder.respond(&wrong, &frame, now, &mut store),
        Err(SyncError::Authentication)
    ));
    for n in 0..frame.len() {
        assert!(responder.respond(&b, &frame[..n], now, &mut store).is_err());
    }
    for i in [3, 4, 6, 9, 11, 18, 23, 24, 30] {
        let mut invalid = frame.clone();
        invalid[i] ^= if i == 3 { 0x80 } else { 1 };
        assert!(responder.respond(&b, &invalid, now, &mut store).is_err());
    }
    assert_eq!(store.calls, 0);
    store.fail = true;
    assert!(matches!(
        responder.respond(&b, &frame, now, &mut store),
        Err(SyncError::StateUnavailable)
    ));
    store.fail = false;
    assert!(responder.respond(&b, &frame, now, &mut store).is_ok());
}

#[cfg(unix)]
#[test]
fn reopened_counters_recover_only_from_a_fresh_response_and_remain_shared_with_group_traffic() {
    use knx::{
        data_secure::{GroupReceiver, GroupSender, ReceiveError, SequenceStore},
        telegram::GroupService,
    };
    use std::{
        fs::{self, File},
        io::{self, Write},
        path::Path,
    };
    struct Disk<'a>(&'a Path);
    impl Disk<'_> {
        fn update<T>(
            &self,
            apply: impl FnOnce(&mut NextSequences) -> io::Result<T>,
        ) -> io::Result<T> {
            let lock = File::open(self.0.join("lock"))?;
            lock.lock()?;
            let path = self.0.join("state");
            let bytes: [u8; 16] = fs::read(&path)?
                .try_into()
                .map_err(|_| io::ErrorKind::InvalidData)?;
            let mut state = NextSequences {
                local: u64::from_be_bytes(bytes[..8].try_into().unwrap()),
                peer: u64::from_be_bytes(bytes[8..].try_into().unwrap()),
            };
            let result = apply(&mut state)?;
            let tmp = self.0.join("pending");
            let mut output = File::create(&tmp)?;
            output.write_all(&state.local.to_be_bytes())?;
            output.write_all(&state.peer.to_be_bytes())?;
            output.sync_all()?;
            fs::rename(tmp, path)?;
            File::open(self.0)?.sync_all()?;
            Ok(result)
        }
    }
    impl SyncStore for Disk<'_> {
        type Error = io::Error;
        fn reconcile(
            &mut self,
            _: u16,
            _: u16,
            f: NextSequences,
            origin: SyncUpdate,
        ) -> io::Result<NextSequences> {
            self.update(|s| {
                if s.local == 0
                    || s.local > MAX_SEQUENCE
                    || (s.peer == 0 && origin == SyncUpdate::PeerRequest)
                {
                    return Err(io::ErrorKind::InvalidData.into());
                }
                s.local = s.local.max(f.local);
                s.peer = s.peer.max(f.peer);
                Ok(*s)
            })
        }
    }
    impl OutgoingSequenceStore for Disk<'_> {
        type Error = io::Error;
        fn reserve(&mut self, _: u16) -> io::Result<Option<u64>> {
            self.update(|s| {
                if s.local == 0 {
                    return Err(io::ErrorKind::InvalidData.into());
                }
                if s.local > MAX_SEQUENCE {
                    return Ok(None);
                }
                let n = s.local;
                s.local += 1;
                Ok(Some(n))
            })
        }
    }
    impl SequenceStore for Disk<'_> {
        type Error = io::Error;
        fn advance(&mut self, _: u16, n: u64) -> io::Result<bool> {
            self.update(|s| {
                if s.peer == 0 {
                    return Err(io::ErrorKind::InvalidData.into());
                }
                if n < s.peer {
                    return Ok(false);
                }
                s.peer = n + 1;
                Ok(true)
            })
        }
    }
    let dir = std::env::temp_dir().join(format!(
        "knx-sync-{}-{:?}",
        std::process::id(),
        std::thread::current().id()
    ));
    fs::create_dir(&dir).unwrap();
    File::create_new(dir.join("lock")).unwrap();
    // Trusted local sequence, missing inbound baseline.
    fs::write(
        dir.join("state"),
        [10u64.to_be_bytes(), 0u64.to_be_bytes()].concat(),
    )
    .unwrap();
    let local = SyncPeer::new(LOCAL, PEER, key()).unwrap();
    let remote = SyncPeer::new(PEER, LOCAL, key()).unwrap();
    let mut b = Store::new(50, 90);
    let now = Instant::now();
    let (unsolicited, _) = remote.request(now, &mut b).unwrap();
    assert!(matches!(
        SyncResponder::new(LOCAL).respond(&local, &ind(&unsolicited), now, &mut Disk(&dir)),
        Err(SyncError::StateUnavailable)
    ));
    let (req, mut pending) = local.request(now, &mut Disk(&dir)).unwrap();
    let resp = SyncResponder::new(PEER)
        .respond(&remote, &ind(&req), now, &mut b)
        .unwrap();
    pending.complete(&ind(&resp), now, &mut Disk(&dir)).unwrap();
    let group = "0/4/0".parse().unwrap();
    let group_key = [0x42; 16];
    let group_req = GroupSender::new(LOCAL, group, group_key)
        .request(GroupService::Read, &mut Disk(&dir))
        .unwrap();
    assert_eq!(group_req.as_bytes()[12..18], [0, 0, 0, 0, 0, 90]);
    let group_sender = GroupSender::new(PEER, group, group_key);
    let receiver = GroupReceiver::new(PEER, group, group_key);
    b.state.local = 50;
    let old = group_sender.request(GroupService::Read, &mut b).unwrap();
    assert!(matches!(
        receiver.receive(&ind(&old), &mut Disk(&dir)),
        Err(ReceiveError::Replay)
    ));
    let fresh = group_sender.request(GroupService::Read, &mut b).unwrap();
    assert_eq!(
        receiver
            .receive(&ind(&fresh), &mut Disk(&dir))
            .unwrap()
            .sequence,
        51
    );
    // Another host can commit group updates while the peer's response is in flight.
    let (req, mut second) = local.request(now, &mut Disk(&dir)).unwrap();
    let lower = SyncResponder::new(PEER)
        .respond(&remote, &ind(&req), now, &mut b)
        .unwrap();
    for _ in 0..3 {
        GroupSender::new(LOCAL, group, group_key)
            .request(GroupService::Read, &mut Disk(&dir))
            .unwrap();
        let newer = group_sender.request(GroupService::Read, &mut b).unwrap();
        receiver.receive(&ind(&newer), &mut Disk(&dir)).unwrap();
    }
    let before = Disk(&dir).update(|state| Ok(*state)).unwrap();
    second.complete(&ind(&lower), now, &mut Disk(&dir)).unwrap();
    assert_eq!(Disk(&dir).update(|state| Ok(*state)).unwrap(), before);
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn credential_rotation_preserves_counters_and_the_device_wide_response_budget() {
    let mut a = Store::new(41, 21);
    let mut b = Store::new(21, 41);
    let now = Instant::now();
    let mut responder = SyncResponder::new(PEER);
    for (i, secret) in [key(), [0x21; 16], key()].into_iter().enumerate() {
        let time = now + Duration::from_secs(i as u64);
        let bound = SyncPeer::new(LOCAL, PEER, secret).unwrap();
        let returning = SyncPeer::new(PEER, LOCAL, secret).unwrap();
        let (packet, mut pending) = bound.request(time, &mut a).unwrap();
        let before = a.state;
        let reply = responder
            .respond(&returning, &ind(&packet), time, &mut b)
            .unwrap();
        pending.complete(&ind(&reply), time, &mut a).unwrap();
        assert!(a.state.local >= before.local && a.state.peer >= before.peer);
        let secondary = SyncPeer::new(0x1103, PEER, [0x33; 16]).unwrap();
        let secondary_in = SyncPeer::new(PEER, 0x1103, [0x33; 16]).unwrap();
        let (another, _) = secondary.request(time, &mut Store::new(1, 1)).unwrap();
        assert!(matches!(
            responder.respond(&secondary_in, &ind(&another), time, &mut b),
            Err(SyncError::RateLimited)
        ));
    }
    assert_eq!(a.state.local, 44);
    assert_eq!(b.state.peer, 43);
}

#[test]
fn discarded_old_challenge_cannot_complete_after_key_replacement_or_restoration() {
    let mut a = Store::new(10, 10);
    let mut b = Store::new(20, 20);
    let now = Instant::now();
    let mut local = SyncPeer::new(LOCAL, PEER, key()).unwrap();
    let remote = SyncPeer::new(PEER, LOCAL, key()).unwrap();
    let delayed = {
        let (request, _pending) = local.request(now, &mut a).unwrap();
        ind(&SyncResponder::new(PEER)
            .respond(&remote, &ind(&request), now, &mut b)
            .unwrap())
    };
    for secret in [[9; 16], key()] {
        local.replace_key(secret);
        let (_, mut fresh) = local.request(now, &mut a).unwrap();
        assert_eq!(
            fresh.complete(&delayed, now, &mut a),
            Err(SyncError::Authentication)
        );
    }
    assert_eq!(a.calls, 0);
}
