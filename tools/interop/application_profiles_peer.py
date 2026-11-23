"""Exchange Rust application-profile group telegrams with offline XKNX 4.1.1 devices."""

import asyncio
import json
import shutil
import subprocess
import tempfile
from pathlib import Path

from xknx import XKNX
from xknx.cemi import CEMIFrame, CEMILData, CEMIMessageCode
from xknx.devices import Cover, ExposeSensor, Light, Sensor, Switch
from xknx.dpt import DPTControlDimming
from xknx.telegram import GroupAddress, IndividualAddress, Telegram
from xknx.telegram.apci import GroupValueRead, GroupValueResponse, GroupValueWrite

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]


def requests(client):
    wire = subprocess.check_output([client, "commands"])
    while wire:
        size = wire[0]
        assert size and size < len(wire)
        frame = CEMIFrame.from_knx(wire[1 : size + 1])
        assert frame.code == CEMIMessageCode.L_DATA_REQ
        yield frame.data.telegram()
        wire = wire[size + 1 :]


def feedback(stack):
    data = CEMILData.init_from_telegram(
        stack.telegrams.get_nowait(), IndividualAddress("1.1.2")
    )
    return CEMIFrame(code=CEMIMessageCode.L_DATA_IND, data=data).to_knx()


async def sunblind_requests():
    stack = XKNX()
    cover = Cover(
        stack, group_address_long="1/2/3", group_address_short="1/2/4", sync_state=False
    )
    expected = bytearray()
    try:
        for action in (
            cover.set_up,
            cover.set_down,
            cover.set_short_up,
            cover.set_short_down,
        ):
            await action()
            data = CEMILData.init_from_telegram(
                stack.telegrams.get_nowait(), IndividualAddress("1.1.1")
            )
            expected.extend(
                CEMIFrame(code=CEMIMessageCode.L_DATA_REQ, data=data).to_knx()
            )
    finally:
        cover.async_remove_tasks()
    return expected


async def compare_temperature_sensors(client):
    stack = XKNX()
    frames = bytearray()
    read = Telegram(destination_address=GroupAddress("2/1/2"), payload=GroupValueRead())

    def indication(telegram, source):
        data = CEMILData.init_from_telegram(telegram, IndividualAddress(source))
        return CEMIFrame(code=CEMIMessageCode.L_DATA_IND, data=data).to_knx()

    for group, source, celsius in [("2/1/1", "1.1.5", -12.5), ("2/1/2", "1.1.6", 21.5)]:
        sensor = ExposeSensor(stack, value_type="9.001", group_address=group)
        await sensor.set(celsius)
        update = stack.telegrams.get_nowait()
        sensor.process_group_write(update)
        if group == "2/1/2":
            sensor.process_group_read(read)
            update = stack.telegrams.get_nowait()
            assert isinstance(update.payload, GroupValueResponse)
        frames.extend(indication(update, source))
    frames.extend(indication(read, "1.1.1"))
    result = await asyncio.to_thread(
        subprocess.check_output, [client, "temperature"], input=frames
    )
    assert len(result) == 26
    for wire, group, source, apci, expected in [
        (result[:13], "2/1/1", "1.1.5", GroupValueWrite, -9.0),
        (result[13:], "2/1/2", "1.1.6", GroupValueResponse, 20.0),
    ]:
        frame = CEMIFrame.from_knx(wire)
        assert frame.code == CEMIMessageCode.L_DATA_REQ
        incoming = frame.data.telegram()
        assert incoming.source_address == IndividualAddress(source)
        assert incoming.destination_address == GroupAddress(group)
        assert isinstance(incoming.payload, apci)
        reader = Sensor(
            stack, value_type="9.001", group_address_state=group, sync_state=False
        )
        reader.process_group_write(incoming)
        assert reader.sensor_value.value == expected


def exchange(client):
    on, off, dimmer_on, relative, absolute = requests(client)
    stack = XKNX()
    device = Switch(
        stack,
        group_address="1/2/2",
        group_address_state="1/2/1",
        respond_to_read=True,
        sync_state=False,
    )
    replies = bytearray()
    for command, state in [(on, True), (off, False)]:
        assert command.destination_address == GroupAddress("1/2/1")
        device.process_group_write(command)
        assert device.state is state
        device.process_group_read(
            Telegram(
                destination_address=GroupAddress("1/2/2"), payload=GroupValueRead()
            )
        )
        replies.extend(feedback(stack))

    light = Light(
        stack,
        group_address_switch="1/2/1",
        group_address_brightness_state="1/2/3",
        group_address_brightness="1/2/4",
        sync_state=False,
    )
    light.process_group_write(dimmer_on)
    assert light.state is True
    # Model a completed quarter-step from zero; XKNX decodes the relative command.
    assert relative.destination_address == GroupAddress("1/2/2")
    step = DPTControlDimming.from_knx(relative.payload.value)
    assert step.control.value == 1 and step.step_code == 3
    light.brightness.set(round(255 / (2 ** (step.step_code - 1))))
    light.process_group_write(stack.telegrams.get_nowait())
    assert light.current_brightness == 64
    assert absolute.destination_address == GroupAddress("1/2/3")
    light.process_group_write(absolute)
    assert light.current_brightness == 128
    light.brightness.respond()
    replies.extend(feedback(stack))
    subprocess.run([client, "feedback"], input=replies, check=True)


if __name__ == "__main__":
    with tempfile.TemporaryDirectory() as folder:
        path = Path(folder)
        (path / "src").mkdir()
        shutil.copyfile(HERE / "application_profiles_client.rs", path / "src/main.rs")
        (path / "Cargo.toml").write_text(
            '[package]\nname="application-profiles-peer"\nversion="0.0.0"\nedition="2024"\n[dependencies]\n'
            f"knx={{path={json.dumps(str(ROOT))}}}\n"
        )
        subprocess.run(["cargo", "build", "--quiet", "--offline"], check=True, cwd=path)
        client = path / "target/debug/application-profiles-peer"
        exchange(client)
        asyncio.run(compare_temperature_sensors(client))
        expected = asyncio.run(sunblind_requests())
        assert subprocess.check_output([client, "shading"]) == expected
    print(
        "Rust and XKNX 4.1.1 verified FB 320/321 temperature writes and read responses, FB 417/418 lighting commands and feedback, and FB 800 sunblind commands."
    )
