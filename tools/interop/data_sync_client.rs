//! Calimero sequence synchronization peer using synthetic installation state.
use knx::data_secure::{
    OutgoingSequenceStore,
    sync::{NextSequences, SyncPeer, SyncResponder, SyncStore, SyncUpdate},
};
use std::{
    io::{self, Read, Write},
    time::Instant,
};
struct Store(NextSequences);
impl OutgoingSequenceStore for Store {
    type Error = ();
    fn reserve(&mut self, local: u16) -> Result<Option<u64>, ()> {
        assert_eq!(local, 0x1101);
        let seq = self.0.local;
        self.0.local += 1;
        Ok(Some(seq))
    }
}
impl SyncStore for Store {
    type Error = ();
    fn reconcile(
        &mut self,
        local: u16,
        peer: u16,
        f: NextSequences,
        _: SyncUpdate,
    ) -> Result<NextSequences, ()> {
        assert_eq!((local, peer), (0x1101, 0x1102));
        self.0.local = self.0.local.max(f.local);
        self.0.peer = self.0.peer.max(f.peer);
        Ok(self.0)
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut store = Store(NextSequences {
        local: 200,
        peer: 1,
    });
    let binding = SyncPeer::new(0x1101, 0x1102, std::array::from_fn(|i| i as u8))?;
    let (req, mut pending) = binding.request(Instant::now(), &mut store)?;
    let mut stdout = io::stdout().lock();
    stdout.write_all(req.as_bytes())?;
    stdout.flush()?;
    let mut frame = [0; 34];
    let mut stdin = io::stdin().lock();
    stdin.read_exact(&mut frame)?;
    pending.complete(&frame, Instant::now(), &mut store)?;
    assert_eq!(
        store.0,
        NextSequences {
            local: 201,
            peer: 100
        }
    );
    stdin.read_exact(&mut frame)?;
    let res = SyncResponder::new(0x1101).respond(&binding, &frame, Instant::now(), &mut store)?;
    stdout.write_all(res.as_bytes())?;
    stdout.flush()?;
    Ok(())
}
