//! Synthetic group sender and receiver paired with XKNX in data_secure_peer.py.
use knx::{
    data_secure::{GroupReceiver, GroupSender, OutgoingSequenceStore, ReceiveError, SequenceStore},
    telegram::{GroupData, GroupService},
};
use std::io::{self, Read, Write};

struct TrialState(u64);
impl SequenceStore for TrialState {
    type Error = ();
    fn advance(&mut self, sender: u16, sequence: u64) -> Result<bool, ()> {
        assert_eq!(sender, 0x5001);
        if sequence <= self.0 {
            return Ok(false);
        }
        self.0 = sequence;
        Ok(true)
    }
}
impl OutgoingSequenceStore for TrialState {
    type Error = ();
    fn reserve(&mut self, sender: u16) -> Result<Option<u64>, ()> {
        assert_eq!(sender, 0x5002);
        self.0 += 1;
        Ok(Some(self.0))
    }
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let group = "0/4/0".parse()?;
    let original = GroupReceiver::new(0x5001, group, std::array::from_fn(|i| i as u8));
    let rotated = GroupReceiver::new(0x5001, group, std::array::from_fn(|i| 16 + i as u8));
    let mut state = TrialState(20);
    let mut input = io::stdin().lock();
    let mut frame = [0; 56];
    let expected: Vec<u8> = (0..32).collect();
    input.read_exact(&mut frame)?;
    assert_eq!(
        original.receive(&frame, &mut state)?.service(),
        GroupService::Write(GroupData::Bytes(&expected))
    );
    input.read_exact(&mut frame)?;
    assert!(matches!(
        rotated.receive(&frame, &mut state),
        Err(ReceiveError::Replay)
    ));
    input.read_exact(&mut frame)?;
    assert!(matches!(
        rotated.receive(&frame, &mut state),
        Err(ReceiveError::Authentication)
    ));
    input.read_exact(&mut frame)?;
    assert_eq!(rotated.receive(&frame, &mut state)?.sequence, 22);
    input.read_exact(&mut frame)?;
    assert!(matches!(
        original.receive(&frame, &mut state),
        Err(ReceiveError::Replay)
    ));
    input.read_exact(&mut frame)?;
    assert_eq!(original.receive(&frame, &mut state)?.sequence, 23);
    let sender = GroupSender::new(0x5002, group, std::array::from_fn(|i| i as u8));
    let replacement = GroupSender::new(0x5002, group, std::array::from_fn(|i| 16 + i as u8));
    let mut outgoing = TrialState(30);
    let mut output = io::stdout().lock();
    let external: Vec<u8> = (0..240).collect();
    for (binding, service) in [
        (&sender, GroupService::Read),
        (&sender, GroupService::Write(GroupData::Inline(1))),
        (
            &replacement,
            GroupService::Response(GroupData::Bytes(&external)),
        ),
        (&sender, GroupService::Write(GroupData::Inline(0))),
    ] {
        let frame = binding.request(service, &mut outgoing)?;
        output.write_all(&(frame.as_bytes().len() as u16).to_be_bytes())?;
        output.write_all(frame.as_bytes())?;
    }
    Ok(())
}
