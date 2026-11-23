use knx::{
    CodecError,
    telegram::{GroupData, GroupService, GroupTelegram, LinkPrimitive},
    temperature::TemperatureSensor,
};

// XKNX 4.1.1 DPTTemperature and CEMILData indications for FB 320 and FB 321.
const OUTSIDE: [u8; 13] = [
    0x29, 0, 0xbc, 0xe0, 0x11, 5, 0x11, 1, 3, 0, 0x80, 0x83, 0x1e,
];
const ROOM: [u8; 13] = [
    0x29, 0, 0xbc, 0xe0, 0x11, 6, 0x11, 2, 3, 0, 0x40, 0x0c, 0x33,
];
const READ: [u8; 11] = [0x29, 0, 0xbc, 0xe0, 0x11, 1, 0x11, 2, 1, 0, 0];

fn room() -> TemperatureSensor {
    TemperatureSensor {
        source: 0x1106,
        read_enabled: true,
        output: "2/1/2".parse().unwrap(),
    }
}
#[test]
fn outside_publish_and_room_response_match_independent_temperature_frames() {
    let outside = TemperatureSensor {
        source: 0x1105,
        read_enabled: false,
        output: "2/1/1".parse().unwrap(),
    };
    let room = room();
    let outdoor = GroupTelegram::parse(&OUTSIDE).unwrap();
    let indoor = GroupTelegram::parse(&ROOM).unwrap();
    assert_eq!(outside.observed_celsius(&outdoor), Ok(Some(-12.5)));
    assert_eq!(room.observed_celsius(&indoor), Ok(Some(21.5)));
    let mut word = [0; 2];
    let publish = outside.publish(-12.5, &mut word).unwrap();
    let mut wire = [0; 13];
    assert_eq!(publish.encode(&mut wire).unwrap(), wire.len());
    assert_eq!(wire[0], 0x11);
    assert_eq!(wire[1..], OUTSIDE[1..]);
    let read = GroupTelegram::parse(&READ).unwrap();
    let response = room
        .respond_to_read(&read, 21.5, &mut word)
        .unwrap()
        .unwrap();
    response.encode(&mut wire).unwrap();
    assert_eq!(wire[0], 0x11);
    assert_eq!(wire[1..], ROOM[1..]);
}
#[test]
fn only_commissioned_temperature_indications_supply_measurements() {
    let sensor = room();
    let mut frame = GroupTelegram::parse(&ROOM).unwrap();
    for (data, error) in [
        (&[0x7f, 0xff][..], CodecError::InvalidData),
        (&[0xf8, 0x00], CodecError::OutsideRange),
        (
            &[0],
            CodecError::PayloadLength {
                expected: 2,
                actual: 1,
            },
        ),
        (
            &[],
            CodecError::PayloadLength {
                expected: 2,
                actual: 0,
            },
        ),
        (
            &[0, 0, 0],
            CodecError::PayloadLength {
                expected: 2,
                actual: 3,
            },
        ),
    ] {
        frame.service = GroupService::Write(GroupData::Bytes(data));
        assert_eq!(sensor.observed_celsius(&frame), Err(error));
    }
    frame.service = GroupService::Write(GroupData::Inline(1));
    assert_eq!(
        sensor.observed_celsius(&frame),
        Err(CodecError::InvalidData)
    );
    frame.source ^= 1;
    assert_eq!(sensor.observed_celsius(&frame), Ok(None));
    frame.source = sensor.source;
    frame.destination = "2/1/1".parse().unwrap();
    assert_eq!(sensor.observed_celsius(&frame), Ok(None));
    frame.destination = sensor.output;
    for primitive in [
        LinkPrimitive::Request,
        LinkPrimitive::Confirmation { successful: true },
    ] {
        frame.primitive = primitive;
        assert_eq!(sensor.observed_celsius(&frame), Ok(None));
    }
    frame.primitive = LinkPrimitive::Indication;
    frame.service = GroupService::Read;
    assert_eq!(sensor.observed_celsius(&frame), Ok(None));
}
#[test]
fn read_handling_and_invalid_samples_preserve_the_output_buffer() {
    let sensor = room();
    let mut read = GroupTelegram::parse(&READ).unwrap();
    let mut data = [42, 42];
    let disabled = TemperatureSensor {
        read_enabled: false,
        ..sensor
    };
    assert_eq!(disabled.respond_to_read(&read, 21.5, &mut data), Ok(None));
    assert_eq!(data, [42, 42]);
    assert_eq!(
        sensor.respond_to_read(&read, f64::NAN, &mut data),
        Err(CodecError::NonFinite)
    );
    assert_eq!(
        sensor.publish(-274., &mut data),
        Err(CodecError::OutsideRange)
    );
    read.primitive = LinkPrimitive::Request;
    assert_eq!(sensor.respond_to_read(&read, 20., &mut data), Ok(None));
    read.primitive = LinkPrimitive::Indication;
    read.destination = "2/1/1".parse().unwrap();
    assert_eq!(sensor.respond_to_read(&read, 20., &mut data), Ok(None));
    read.destination = sensor.output;
    read.service = GroupService::Write(GroupData::Inline(1));
    assert_eq!(sensor.respond_to_read(&read, 20., &mut data), Ok(None));
    assert_eq!(data, [42, 42]);
}

#[test]
fn both_temperature_profiles_share_the_full_dpt_range_and_services() {
    for (source, group) in [(0x1105, "2/1/1"), (0x1106, "2/1/2")] {
        let sensor = TemperatureSensor {
            source,
            output: group.parse().unwrap(),
            read_enabled: true,
        };
        let mut read = GroupTelegram::request(0x1101, sensor.output, GroupService::Read);
        read.primitive = LinkPrimitive::Indication;
        let mut payload = [0; 2];
        for (input, observed) in [(-273., -272.96), (670_433.28, 670_433.28)] {
            let mut written = sensor.publish(input, &mut payload).unwrap();
            written.primitive = LinkPrimitive::Indication;
            assert_eq!(sensor.observed_celsius(&written), Ok(Some(observed)));
            let mut answer = sensor
                .respond_to_read(&read, input, &mut payload)
                .unwrap()
                .unwrap();
            answer.primitive = LinkPrimitive::Indication;
            assert_eq!(sensor.observed_celsius(&answer), Ok(Some(observed)));
        }
    }
}
