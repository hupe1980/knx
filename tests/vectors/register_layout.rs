use knx::{
    registers::{RegisterFields, RegisterLayout},
    text_time::DateTime,
};

fn fixture(count: usize, dated: bool) -> Vec<u8> {
    let mut wire = Vec::new();
    if dated {
        wire.extend([0, 0, 0, 0, 0, 0, 128, 0]);
    }
    wire.extend(b"ERL-METER01\0");
    for n in 0..count {
        // Signed two's complement counts from –256 upward, supplier-specific selector 17.
        wire.extend([255, 255, 255, n as u8, 17, 0]);
    }
    wire
}

macro_rules! layout_vectors {
    ($(($id:literal,$n:literal,$dated:literal,$tx:ident,$rx:ident)),+) => {
        $(
            #[test] fn $tx() {
                let layout=RegisterLayout::from_dpt($id).unwrap();
                let records:Vec<_>=(0..$n).map(|i|RegisterFields{count:Some(-256+i),selector:Some(17)}).collect();
                let mut output=vec![0;layout.payload_len()];
                let timestamp=if $dated {Some(DateTime::Fault)}else{None};
                assert_eq!(layout.encode("ERL-METER01",timestamp,&records,&mut output).unwrap(),fixture($n,$dated));
            }
            #[test] fn $rx() {
                let layout=RegisterLayout::from_dpt($id).unwrap();
                let bytes=fixture($n,$dated);
                let decoded=layout.decode(&bytes).unwrap();
                assert_eq!(decoded.label,"ERL-METER01");
                assert_eq!(decoded.timestamp,if $dated {Some(DateTime::Fault)}else{None});
                assert_eq!(decoded.registers().len(),$n);
                for (i,item) in decoded.registers().enumerate(){
                    assert_eq!(item,RegisterFields{count:Some(-256+i as i32),selector:Some(17)});
                }
            }
        )+
        pub(super) const EVIDENCE:&[(&str,usize,&str)] = &[
            $(($id,super::ENCODE,concat!("tests/inventory.rs::register_layout::",stringify!($tx))),
              ($id,super::DECODE,concat!("tests/inventory.rs::register_layout::",stringify!($rx))),)+
        ];
    };
}
layout_vectors!(
    ("277.1200", 4, false, dpt_277_tx, dpt_277_rx),
    ("278.1200", 5, false, dpt_278_tx, dpt_278_rx),
    ("279.1200", 6, false, dpt_279_tx, dpt_279_rx),
    ("280.1200", 11, false, dpt_280_tx, dpt_280_rx),
    ("281.1200", 4, true, dpt_281_tx, dpt_281_rx),
    ("282.1200", 5, true, dpt_282_tx, dpt_282_rx),
    ("283.1200", 6, true, dpt_283_tx, dpt_283_rx),
    ("284.1200", 11, true, dpt_284_tx, dpt_284_rx)
);
