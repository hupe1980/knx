use knx::{lighting::SwitchActuator, telegram::GroupTelegram};
#[test]
fn commanded_and_observed_states_have_separate_evidence() {
    let actor = SwitchActuator {
        command: "1/2/1".parse().unwrap(),
        feedback: Some((0x1102, "1/2/2".parse().unwrap())),
    };
    assert_eq!(actor.observed(&actor.set(0, true)), Ok(None));
    let wire = [0x29, 0, 0xbc, 0xe0, 0x11, 2, 0x0a, 2, 1, 0, 0x81];
    let observed = |p: &[u8]| actor.observed(&GroupTelegram::parse(p).unwrap());
    assert_eq!(observed(&wire), Ok(Some(true)));
    for (i, value, expected) in [
        (0, 0x2e, None),
        (5, 3, None),
        (7, 1, None),
        (10, 0, None),
        (10, 0x40, Some(false)),
    ] {
        let mut changed = wire;
        changed[i] = value;
        assert_eq!(observed(&changed), Ok(expected));
    }
    let mut invalid = wire;
    invalid[10] = 0x82;
    assert!(observed(&invalid).is_err());
    let mut octets = wire.to_vec();
    octets[8] = 2;
    octets[10] = 0x80;
    octets.push(1);
    assert!(observed(&octets).is_err());
    assert_eq!(
        SwitchActuator {
            feedback: None,
            ..actor
        }
        .observed(&GroupTelegram::parse(&wire).unwrap()),
        Ok(None)
    );
}

#[test]
fn mandatory_dimmer_commands_and_actual_level_match_calimero() {
    use knx::{Dimming, StepSize, encode_scaling_percent, lighting::DimmingActuator};
    let groups = ["1/2/1", "1/2/2", "1/2/3", "1/2/4"].map(|s| s.parse().unwrap());
    let [switch, relative, absolute, feedback] = groups;
    let bare = DimmingActuator::new(switch, relative, absolute).unwrap();
    let dimmer = bare.with_level_feedback(0x1102, feedback).unwrap();
    for args in [
        (switch, switch, absolute),
        (switch, relative, switch),
        (switch, relative, relative),
    ] {
        assert!(DimmingActuator::new(args.0, args.1, args.2).is_err());
    }
    assert!(bare.with_level_feedback(0x1102, relative).is_err());
    let half = encode_scaling_percent(50.).unwrap();
    let commands = [
        (
            dimmer.set(0x1102, true),
            &[0x11, 0, 0xbc, 0xe0, 0x11, 2, 0xa, 1, 1, 0, 0x81][..],
        ),
        (
            dimmer.dim(
                0x1102,
                Dimming::Increase(StepSize::from_denominator(4).unwrap()),
            ),
            &[0x11, 0, 0xbc, 0xe0, 0x11, 2, 0xa, 2, 1, 0, 0x8b][..],
        ),
        (
            dimmer.set_level(0x1102, &half),
            &[0x11, 0, 0xbc, 0xe0, 0x11, 2, 0xa, 3, 2, 0, 0x80, 0x80][..],
        ),
    ];
    for (cmd, expected) in commands {
        let mut buf = [0; 16];
        let n = cmd.encode(&mut buf).unwrap();
        assert_eq!(&buf[..n], expected);
        assert_eq!(dimmer.observed_level(&cmd), Ok(None));
    }
    let frame = [0x29, 0, 0x9c, 0xe0, 0x11, 2, 0xa, 4, 2, 0, 0x80, 0x80];
    let observed = GroupTelegram::parse(&frame).unwrap();
    assert_eq!(bare.observed_level(&observed), Ok(None));
    assert_eq!(
        dimmer.observed_level(&observed),
        Ok(Some(128. * 100. / 255.))
    );
    // XKNX Light returned this response to the simulated actual-level query.
    let xknx = [0x29, 0, 0xbc, 0xe0, 0x11, 2, 0x0a, 4, 2, 0, 0x40, 0x80];
    assert_eq!(
        dimmer.observed_level(&GroupTelegram::parse(&xknx).unwrap()),
        Ok(Some(128. * 100. / 255.))
    );
    for (index, byte) in [(0, 0x2e), (5, 3), (7, 3)] {
        let mut wrong = frame;
        wrong[index] = byte;
        assert_eq!(
            dimmer.observed_level(&GroupTelegram::parse(&wrong).unwrap()),
            Ok(None)
        );
    }
    let mut inline = frame[..11].to_vec();
    inline[8] = 1;
    assert!(
        dimmer
            .observed_level(&GroupTelegram::parse(&inline).unwrap())
            .is_err()
    );
    let mut excess = frame.to_vec();
    excess[8] = 3;
    excess.push(0);
    assert!(
        dimmer
            .observed_level(&GroupTelegram::parse(&excess).unwrap())
            .is_err()
    );
}

#[test]
fn xknx_virtual_switch_responds_to_the_two_commissioned_groups() {
    // XKNX 4.1.1 Switch, respond_to_read enabled. Independent fixtures for both states.
    let binding = SwitchActuator {
        command: "1/2/1".parse().unwrap(),
        feedback: Some((0x1102, "1/2/2".parse().unwrap())),
    };
    for on in [true, false] {
        let mut cmd = [0; 11];
        binding.set(0x1101, on).encode(&mut cmd).unwrap();
        assert_eq!(
            cmd,
            [
                0x11,
                0,
                0xbc,
                0xe0,
                0x11,
                1,
                0x0a,
                1,
                1,
                0,
                0x80 + u8::from(on)
            ]
        );
        let response = [
            0x29,
            0,
            0xbc,
            0xe0,
            0x11,
            2,
            0x0a,
            2,
            1,
            0,
            0x40 + u8::from(on),
        ];
        assert_eq!(
            binding.observed(&GroupTelegram::parse(&response).unwrap()),
            Ok(Some(on))
        );
    }
}
