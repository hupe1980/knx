"""Reconcile the release CSV with the KNX DPT v02.02.01 PDF (requires pdftotext)."""
import argparse
import csv
import hashlib
import io
import re
import subprocess
from pathlib import Path

EDITION = "02.02.01"
DIGEST = "72f924a6c793314d1e20573d71f697f07ce76072525b80c53ee0f68e3b7190af"
CATALOG = Path(__file__).resolve().parents[1] / "site/static/standards.csv"
ID = r"(\d{1,3}\.\d{3,4})(?![\d.])"
# Independent TX/RX claims. Each nonempty suffix resolves to a public vector test.
LAYOUT_CLAIMS = {
    f"{major}.1200": (f"register_layout::dpt_{major}_tx", f"register_layout::dpt_{major}_rx")
    for major in range(277, 285)
}
LAYOUT_CLAIMS["230.1000"] = ("mbus_address_tx", "mbus_address_rx")

CLAIMS = {
    "207.100": ("report_burner_status_tx", "report_burner_status_rx"),
    "207.101": ("report_lock_signal_tx", "report_lock_signal_rx"),
    "207.102": ("report_boiler_demand_tx", "report_boiler_demand_rx"),
    "207.104": ("report_actuator_demand_tx", "report_actuator_demand_rx"),
    "207.105": ("report_actuator_status_tx", "report_actuator_status_rx"),
    "209.100": ("report_heat_manager_status_tx", "report_heat_manager_status_rx"),
    "209.101": ("report_room_demand_tx", "report_room_demand_rx"),
    "209.102": ("report_cooling_manager_status_tx", "report_cooling_manager_status_rx"),
    "209.103": ("report_water_controller_status_tx", "report_water_controller_status_rx"),
    "210.100": ("report_flow_demand_tx", "report_flow_demand_rx"),
    "214.100": ("report_heat_power_demand_tx", "report_heat_power_demand_rx"),
    "214.101": ("report_cool_power_demand_tx", "report_cool_power_demand_rx"),
    "215.100": ("report_boiler_status_tx", "report_boiler_status_rx"),
    "215.101": ("report_chiller_status_tx", "report_chiller_status_rx"),
    "216.100": ("report_heat_producer_spec_tx", "report_heat_producer_spec_rx"),
    "206.100": ("hvac_206_100_tx", "hvac_206_100_rx"),
    "206.102": ("hvac_206_102_tx", "hvac_206_102_rx"),
    "206.104": ("hvac_206_104_tx", "hvac_206_104_rx"),
    "206.105": ("hvac_206_105_tx", "hvac_206_105_rx"),
    "220.100": ("hvac_220_100_tx", "hvac_220_100_rx"),
    "212.100": ("hvac_212_100_tx", "hvac_212_100_rx"),
    "212.101": ("hvac_212_101_tx", "hvac_212_101_rx"),
    "213.100": ("hvac_213_100_tx", "hvac_213_100_rx"),
    "213.101": ("hvac_213_101_tx", "hvac_213_101_rx"),
    "213.102": ("hvac_213_102_tx", "hvac_213_102_rx"),
    "222.100": ("hvac_222_100_tx", "hvac_222_100_rx"),
    "222.101": ("hvac_222_101_tx", "hvac_222_101_rx"),
    "211.100": ("hvac_211_100_tx", "hvac_211_100_rx"),
    "223.100": ("hvac_223_100_tx", "hvac_223_100_rx"),
    "224.100": ("hvac_224_100_tx", "hvac_224_100_rx"),
    "20.1000": ("communication_mode_tx", "communication_mode_rx"),
    "20.1001": ("add_info_type_tx", "add_info_type_rx"),
    "20.1002": ("radio_mode_tx", "radio_mode_rx"),
    "20.1003": ("radio_filter_tx", "radio_filter_rx"),
    "20.1004": ("system_medium_tx", "system_medium_rx"),
    "20.1005": ("button_function_tx", "button_function_rx"),
    "21.1000": ("radio_modes_tx", "radio_modes_rx"),
    "21.1001": ("radio_filters_tx", "radio_filters_rx"),
    "21.1002": ("security_report_tx", "security_report_rx"),
    "22.1000": ("supported_media_tx", "supported_media_rx"),
    "21.100": ("heat_forcing_tx", "heat_forcing_rx"),
    "21.101": ("cool_forcing_tx", "cool_forcing_rx"),
    "21.102": ("room_heating_status_tx", "room_heating_status_rx"),
    "21.103": ("solar_hot_water_status_tx", "solar_hot_water_status_rx"),
    "21.104": ("fuel_types_tx", "fuel_types_rx"),
    "21.105": ("room_cooling_status_tx", "room_cooling_status_rx"),
    "21.106": ("ventilation_status_tx", "ventilation_status_rx"),
    "22.100": ("hot_water_status_tx", "hot_water_status_rx"),
    "22.101": ("room_climate_status_tx", "room_climate_status_rx"),
    "25.1000": ("repetition_counts_tx", "repetition_counts_rx"),
    "21.1010": ("channels_8_tx", "channels_8_rx"),
    "22.1010": ("channels_16_tx", "channels_16_rx"),
    "30.1010": ("channels_24_tx", "channels_24_rx"),
    "21.601": ("light_faults_tx", "light_faults_rx"),
    "237.600": ("gear_diagnostic_tx", "gear_diagnostic_rx"),
    "238.600": ("dali_diagnostic_tx", "dali_diagnostic_rx"),
    "246.600": ("battery_info_tx", "battery_info_rx"),
    "247.600": ("converter_test_info_tx", "converter_test_info_rx"),
    "244.600": ("converter_status_tx", "converter_status_rx"),
    "245.600": ("converter_results_tx", "converter_results_rx"),
    "276.1200": ("erl_status_tx", "erl_status_rx"),
    "271.1200": ("tariff_day_tx", "tariff_day_rx"),
    "248.600": ("converter_fixed_tx", "converter_fixed_rx"),
    "272.600": ("converter_runtime_tx", "converter_runtime_rx"),
    "1.100": ("hvac_1_100_tx", "hvac_1_100_rx"),
    "20.100": ("hvac_20_100_tx", "hvac_20_100_rx"),
    "20.101": ("hvac_20_101_tx", "hvac_20_101_rx"),
    "20.103": ("hvac_20_103_tx", "hvac_20_103_rx"),
    "20.104": ("hvac_20_104_tx", "hvac_20_104_rx"),
    "20.105": ("hvac_20_105_tx", "hvac_20_105_rx"),
    "20.106": ("hvac_20_106_tx", "hvac_20_106_rx"),
    "20.107": ("hvac_20_107_tx", "hvac_20_107_rx"),
    "20.108": ("hvac_20_108_tx", "hvac_20_108_rx"),
    "20.109": ("hvac_20_109_tx", "hvac_20_109_rx"),
    "20.110": ("hvac_20_110_tx", "hvac_20_110_rx"),
    "20.111": ("hvac_20_111_tx", "hvac_20_111_rx"),
    "20.112": ("hvac_20_112_tx", "hvac_20_112_rx"),
    "20.113": ("hvac_20_113_tx", "hvac_20_113_rx"),
    "20.114": ("hvac_20_114_tx", "hvac_20_114_rx"),
    "20.115": ("hvac_20_115_tx", "hvac_20_115_rx"),
    "20.120": ("hvac_20_120_tx", "hvac_20_120_rx"),
    "20.121": ("hvac_20_121_tx", "hvac_20_121_rx"),
    "20.122": ("hvac_20_122_tx", "hvac_20_122_rx"),
    "23.102": ("hvac_23_102_tx", "hvac_23_102_rx"),
    "31.101": ("hvac_31_101_tx", "hvac_31_101_rx"),
    "7.600": ("colour_kelvin_tx", "colour_kelvin_rx"),
    "12.1200": ("volume_litres_tx", "volume_litres_rx"),
    "12.1201": ("volume_cubic_metres_tx", "volume_cubic_metres_rx"),
    "13.1200": ("delta_litres_tx", "delta_litres_rx"),
    "13.1201": ("delta_cubic_metres_tx", "delta_cubic_metres_rx"),
    "14.1200": ("flow_m3h_tx", "flow_m3h_rx"),
    "14.1201": ("flow_ls_tx", "flow_ls_rx"),
    "242.600": ("xy_colour_tx", "xy_colour_rx"),
    "243.600": ("xy_transition_tx", "xy_transition_rx"),
    "249.600": ("white_transition_tx", "white_transition_rx"),
    "250.600": ("white_control_tx", "white_control_rx"),
    "252.600": ("rgbw_control_tx", "rgbw_control_rx"),
    "253.600": ("xy_control_tx", "xy_control_rx"),
    "254.600": ("rgb_steps_tx", "rgb_steps_rx"),
    "273.001": ("forecast_temp_tx", "forecast_temp_rx"),
    "273.002": ("forecast_wind_tx", "forecast_wind_rx"),
    "273.003": ("forecast_relative_humidity_tx", "forecast_relative_humidity_rx"),
    "273.004": ("forecast_absolute_humidity_tx", "forecast_absolute_humidity_rx"),
    "273.005": ("forecast_co2_tx", "forecast_co2_rx"),
    "273.006": ("forecast_pollutants_tx", "forecast_pollutants_rx"),
    "273.007": ("forecast_sun_tx", "forecast_sun_rx"),
    "274.001": ("forecast_direction_tx", "forecast_direction_rx"),
    "1.1200": ("meter_role_tx", "meter_role_rx"),
    "1.1201": ("energy_direction_tx", "energy_direction_rx"),
    "20.600": ("light_lock_behavior_tx", "light_lock_behavior_rx"),
    "20.601": ("bus_power_behavior_tx", "bus_power_behavior_rx"),
    "20.603": ("blink_mode_tx", "blink_mode_rx"),
    "20.604": ("light_control_mode_tx", "light_control_mode_rx"),
    "20.605": ("switch_input_mode_tx", "switch_input_mode_rx"),
    "20.606": ("switch_button_action_tx", "switch_button_action_rx"),
    "20.607": ("dimming_input_mode_tx", "dimming_input_mode_rx"),
    "20.608": ("switch_on_mode_tx", "switch_on_mode_rx"),
    "20.609": ("load_type_set_tx", "load_type_set_rx"),
    "20.610": ("detected_load_tx", "detected_load_rx"),
    "20.611": ("converter_test_control_tx", "converter_test_control_rx"),
    "20.612": ("converter_control_tx", "converter_control_rx"),
    "20.613": ("converter_data_request_tx", "converter_data_request_rx"),
    "20.801": ("blind_exception_behavior_tx", "blind_exception_behavior_rx"),
    "20.802": ("blind_lock_behavior_tx", "blind_lock_behavior_rx"),
    "20.803": ("blind_input_mode_tx", "blind_input_mode_rx"),
    "20.804": ("blind_control_mode_tx", "blind_control_mode_rx"),
    "20.1202": ("gas_measurement_condition_tx", "gas_measurement_condition_rx"),
    "20.1203": ("breaker_status_tx", "breaker_status_rx"),
    "20.1204": ("euridis_status_tx", "euridis_status_rx"),
    "20.1206": ("peak_notice_tx", "peak_notice_rx"),
    "20.1207": ("peak_event_tx", "peak_event_rx"),
    "20.1208": ("tic_type_tx", "tic_type_rx"),
    "20.1209": ("tic_channel_tx", "tic_channel_rx"),
    "20.1200": ("breaker_valve_state_tx", "breaker_valve_state_rx"),
    "265.1200": ("meter_role_dated_tx", "meter_role_dated_rx"),
    "265.1201": ("energy_direction_dated_tx", "energy_direction_dated_rx"),
    "268.1203": ("breaker_status_dated_tx", "breaker_status_dated_rx"),
    "268.1204": ("euridis_status_dated_tx", "euridis_status_dated_rx"),
    "268.1206": ("peak_notice_dated_tx", "peak_notice_dated_rx"),
    "20.1205": ("plc_status_tx", "plc_status_rx"),
    "268.1205": ("plc_status_dated_tx", "plc_status_dated_rx"),
    "21.1200": ("contact_bank_tx", "contact_bank_rx"),
    "21.1201": ("phase_presence_tx", "phase_presence_rx"),
    "20.602": ("fade_tenths_tx", "fade_tenths_rx"),
    "240.800": ("blind_position_tx", "blind_position_rx"),
    "241.800": ("blind_status_tx", "blind_status_rx"),
    "232.600": ("rgb_tx", "rgb_rx"),
    "251.600": ("rgbw_tx", "rgbw_rx"),
    "257.1200": ("current_phases_tx", "current_phases_rx"),
    "270.1200": ("current_dated_phases_tx", "current_dated_phases_rx"),
    "257.1201": ("potential_phases_tx", "potential_phases_rx"),
    "270.1201": ("potential_dated_phases_tx", "potential_dated_phases_rx"),
    "257.1202": ("apparent_power_phases_tx", "apparent_power_phases_rx"),
    "270.1202": ("apparent_power_dated_phases_tx", "apparent_power_dated_phases_rx"),
    "207.600": ("lighting_status_tx", "lighting_status_rx"),
    "269.1200": ("dated_energy_tx", "dated_energy_rx"),
    "1.002": ("binary_2_tx", "binary_2_rx"),
    "1.003": ("binary_3_tx", "binary_3_rx"),
    "1.004": ("binary_4_tx", "binary_4_rx"),
    "1.005": ("binary_5_tx", "binary_5_rx"),
    "1.006": ("binary_6_tx", "binary_6_rx"),
    "1.007": ("binary_7_tx", "binary_7_rx"),
    "1.008": ("binary_8_tx", "binary_8_rx"),
    "1.009": ("binary_9_tx", "binary_9_rx"),
    "1.010": ("binary_10_tx", "binary_10_rx"),
    "1.011": ("binary_11_tx", "binary_11_rx"),
    "1.012": ("binary_12_tx", "binary_12_rx"),
    "1.013": ("binary_13_tx", "binary_13_rx"),
    "1.014": ("binary_14_tx", "binary_14_rx"),
    "1.015": ("binary_15_tx", "binary_15_rx"),
    "1.016": ("binary_16_tx", "binary_16_rx"),
    "1.017": ("binary_17_tx", "binary_17_rx"),
    "1.018": ("binary_18_tx", "binary_18_rx"),
    "1.019": ("binary_19_tx", "binary_19_rx"),
    "1.021": ("binary_21_tx", "binary_21_rx"),
    "1.022": ("binary_22_tx", "binary_22_rx"),
    "1.023": ("binary_23_tx", "binary_23_rx"),
    "1.024": ("binary_24_tx", "binary_24_rx"),
    "7.001": ("pulse_count_u16_tx", "pulse_count_u16_rx"),
    "7.010": ("property_data_type_tx", "property_data_type_rx"),
    "7.011": ("length_mm_tx", "length_mm_rx"),
    "7.013": ("parameter_lux_tx", "parameter_lux_rx"),
    "8.001": ("pulse_count_i16_tx", "pulse_count_i16_rx"),
    "8.011": ("rotation_degrees_tx", "rotation_degrees_rx"),
    "8.012": ("altitude_m_tx", "altitude_m_rx"),
    "12.001": ("pulse_count_u32_tx", "pulse_count_u32_rx"),
    "12.100": ("operating_seconds_tx", "operating_seconds_rx"),
    "12.101": ("operating_minutes_tx", "operating_minutes_rx"),
    "12.102": ("operating_hours_tx", "operating_hours_rx"),
    "13.001": ("pulse_count_i32_tx", "pulse_count_i32_rx"),
    "13.002": ("flow_rate_ticks_tx", "flow_rate_ticks_rx"),
    "13.100": ("operating_delta_seconds_tx", "operating_delta_seconds_rx"),
    "7.012": ("supply_ma_tx", "supply_ma_rx"),
    "8.010": ("percent_hundredths_tx", "percent_hundredths_rx"),
    "20.102": ("hvac_mode_tx", "hvac_mode_rx"),
    "235.001": ("tariff_energy_tx", "tariff_energy_rx"),
    "7.002": ("period_ms_tx", "period_ms_rx"),
    "7.003": ("period_10ms_tx", "period_10ms_rx"),
    "7.004": ("period_100ms_tx", "period_100ms_rx"),
    "7.005": ("period_s_tx", "period_s_rx"),
    "7.006": ("period_min_tx", "period_min_rx"),
    "7.007": ("period_h_tx", "period_h_rx"),
    "8.002": ("delta_ms_tx", "delta_ms_rx"),
    "8.003": ("delta_10ms_tx", "delta_10ms_rx"),
    "8.004": ("delta_100ms_tx", "delta_100ms_rx"),
    "8.005": ("delta_s_tx", "delta_s_rx"),
    "8.006": ("delta_min_tx", "delta_min_rx"),
    "8.007": ("delta_h_tx", "delta_h_rx"),
    "6.001": ("signed_percent_tx", "signed_percent_rx"),
    "6.010": ("signed_pulses_tx", "signed_pulses_rx"),
    "6.020": ("status_mode_tx", "status_mode_rx"),
    "5.003": ("angle_tx", "angle_rx"),
    "5.005": ("factor_tx", "factor_rx"),
    "5.006": ("tariff_tx", "tariff_rx"),
    "5.010": ("pulse_count_tx", "pulse_count_rx"),
    "2.001": ("priority_switch_tx", "priority_switch_rx"),
    "2.002": ("priority_boolean_tx", "priority_boolean_rx"),
    "2.003": ("priority_enable_tx", "priority_enable_rx"),
    "2.004": ("priority_ramp_tx", "priority_ramp_rx"),
    "2.005": ("priority_alarm_tx", "priority_alarm_rx"),
    "2.006": ("priority_binaryvalue_tx", "priority_binaryvalue_rx"),
    "2.007": ("priority_step_tx", "priority_step_rx"),
    "2.008": ("priority_updown_tx", "priority_updown_rx"),
    "2.009": ("priority_openclose_tx", "priority_openclose_rx"),
    "2.010": ("priority_start_tx", "priority_start_rx"),
    "2.011": ("priority_state_tx", "priority_state_rx"),
    "2.012": ("priority_invert_tx", "priority_invert_rx"),
    "13.011": ("vah_32_tx", "vah_32_rx"),
    "13.012": ("varh_32_tx", "varh_32_rx"),
    "13.013": ("kwh_32_tx", "kwh_32_rx"),
    "13.014": ("kvah_32_tx", "kvah_32_rx"),
    "13.015": ("kvarh_32_tx", "kvarh_32_rx"),
    "13.016": ("mwh_32_tx", "mwh_32_rx"),
    "29.010": ("wh_64_tx", "wh_64_rx"),
    "29.011": ("vah_64_tx", "vah_64_rx"),
    "29.012": ("varh_64_tx", "varh_64_rx"),
    "3.007": ("dimming_tx", "dimming_rx"),
    "3.008": ("blinds_tx", "blinds_rx"),
    "17.001": ("scene_number_tx", "scene_number_rx"),
    "18.001": ("scene_control_tx", "scene_control_rx"),
    "1.001": ("switch_tx", "switch_rx"),
    "5.001": ("scaling_tx", "scaling_rx"),
    "5.004": ("whole_percent_tx", "whole_percent_rx"),
    "9.001": ("temperature_tx", "temperature_rx"),
    "13.010": ("energy_tx", "energy_rx"),
    "4.001": ("ascii_char_tx", "ascii_char_rx"),
    "4.002": ("latin_char_tx", "latin_char_rx"),
    "10.001": ("time_of_day_tx", "time_of_day_rx"),
    "11.001": ("calendar_date_tx", "calendar_date_rx"),
    "16.000": ("fixed_ascii_tx", "fixed_ascii_rx"),
    "16.001": ("fixed_latin_tx", "fixed_latin_rx"),
    "19.001": ("calendar_fields_tx", "calendar_fields_rx"),
    "24.001": ("variable_latin_tx", "variable_latin_rx"),
    "28.001": ("variable_utf8_tx", "variable_utf8_rx"),
    "14.056": ("power_tx", "power_rx"),
}

# The assigned subtypes in §3.10 and §3.15 share a wire representation within each family.
for major, subtypes, family in (
    (9, (*range(2, 12), *range(20, 31)), "short_float"),
    (14, (*range(56), *range(57, 81)), "long_float"),
):
    for subtype in subtypes:
        test = f"{family}::dpt_{major}_{subtype:03}"
        CLAIMS[f"{major}.{subtype:03}"] = (f"{test}_tx", f"{test}_rx")


for key in [
    "15.000", "20.001", "20.002", "20.003", "20.004", "20.005", "20.006", "20.007",
    "20.008", "20.011", "20.012", "20.013", "20.014", "20.017", "20.020", "20.021",
    "20.022", "21.001", "21.002", "23.001", "23.002", "23.003", "26.001", "27.001",
    "202.001", "202.002", "203.002", "203.003", "203.004", "203.005", "203.006", "203.007",
    "203.011", "203.012", "203.013", "203.014", "203.015", "203.017", "204.001", "205.002",
    "205.003", "205.004", "205.005", "205.006", "205.007", "205.017", "217.001", "218.001",
    "218.002", "219.001", "221.001", "225.001", "225.002", "225.003", "229.001", "231.001",
    "234.001", "200.100", "200.101", "201.100", "201.102", "201.104", "201.105", "201.107",
    "201.108", "201.109", "203.100", "203.101", "203.102", "203.104", "205.100", "205.101",
    "205.102", "205.103", "234.002", "236.001", "238.001", "239.001", "255.001", "256.001",
    "265.001", "265.005", "265.009", "265.011", "265.012", "266.027", "266.056", "266.080",
    "267.001",
]:
    test = f"structured_vectors::dpt_{key.replace('.', '_')}"
    CLAIMS[key] = (f"{test}_tx", f"{test}_rx")


def owner(section):
    major, minor = map(int, section.split("."))
    if major > 3:
        return {4: "hvac", 5: "load-management", 6: "lighting", 7: "blinds",
                8: "system", 9: "metering", 10: "weather"}[major]
    if minor in (4, 11, 12, 17, 20, 24, 27):
        return "text-calendar"
    if minor in (1, 2, 3, 5, 6, 8, 9, 10, 13, 14, 15, 18, 19, 28):
        return "scalar-control"
    return "structured-values"


def collect(text):
    overview, rest = text.split("\n2 Overview\n")[1].split("\n3 Datapoint Types for common use\n")
    body, parameters = rest.split("\n11 Parameter Types\n")
    names = {}
    for line in overview.splitlines():
        match = re.match(r"^\s*" + ID + r"\s+.*?\b(DPT_\S+)\s*$", line)
        if match:
            key, name = match.groups()
            if key in names:
                raise ValueError(f"duplicate overview identifier {key}")
            names[key] = name
        elif line.strip() == "ce_Status":
            # The table wraps this identifier at its column boundary.
            names["268.1204"] += line.strip()
    sections = {}
    for line in body.splitlines():
        heading = re.match(r"^\s*((?:[3-9]|10)\.\d{1,2})\s+\S", line)
        if heading:
            section = heading[1]
        row = re.match(r"^\s*" + ID + r"\s+(.*)", line)
        if row and (row[1] in names or re.match(r"DPT_", row[2])):
            sections.setdefault(row[1], set()).add(section)
    if set(names) - sections.keys() or sections.keys() - names.keys() != {"249.600"}:
        raise ValueError("chapter 2 and the detailed tables need reconciliation")
    names["249.600"] = "DPT_Brightness_Colour_Temperature_Transition"
    # These IDs occur both in a reference/example table and their definition.
    definitions = {"9.028": "3.10", "14.056": "3.15", "20.014": "3.21"}
    for key in sorted(names, key=lambda k: tuple(map(int, k.split(".")))):
        choices = sections[key]
        if len(choices) > 1 and key not in definitions:
            raise ValueError(f"ambiguous definition for {key}: {choices}")
        section = definitions.get(key, next(iter(choices)))
        if section not in choices:
            raise ValueError(f"missing definition of {key} in {section}")
        tx, rx = CLAIMS.get(key, LAYOUT_CLAIMS.get(key, ("", "")))
        yield ("dpt", key, names[key], section, owner(section), tx, rx, "")
    seen = set()
    table, annex = parameters.split("DPT_HVACStatus is a non-standard DPT", 1)
    for line in table.splitlines():
        match = re.match(r"^(PART_\S+)\s+", line)
        if not match:
            continue
        assignment = re.search(r"\b([0-9A-F]{4}h)\s+(\S+)", line)
        if assignment:
            key, binding = assignment.groups()
            if key in seen:
                raise ValueError(f"duplicate parameter {key}")
            seen.add(key)
            yield ("parameter", key, match[1], "11", "parameters", "", "", binding)
        else:
            binding = re.search(r"\s(\d{1,3}\.\d{3,4})\s", line)
            yield ("unassigned-parameter", "", match[1], "11", "parameters", "", "",
                   binding[1] if binding else "")
    yield ("legacy", "", "DPT_HVACStatus", "A", "hvac", "", "", "")
    for section, name in re.findall(r"^(B\.2\.\d+) .+? \((\w+)\) \(LEGACY!\)$", annex, re.MULTILINE):
        yield ("legacy", "", name, section, "lighting", "", "", "")


def render(text):
    out = io.StringIO()
    writer = csv.writer(out, lineterminator="\n")
    writer.writerow(("edition", "kind", "id", "name", "section", "owner", "encode", "decode",
                     "encode_evidence", "decode_evidence", "parameter_binding", "parameter_validation"))
    for kind, key, name, section, team, tx, rx, binding in collect(text):
        if kind in ("parameter", "unassigned-parameter"):
            if kind == "unassigned-parameter" and name != "PART_Input_Connected":
                scope = "unspecified"
            elif key in ("E401h", "1705h"):
                scope = "layout"
            else:
                scope = "value"
            writer.writerow((EDITION, kind, key, name, section, team, "not-applicable",
                             "not-applicable", "", "", binding, scope))
            continue
        evidence = [f"tests/inventory.rs::{test}" if test else "" for test in (tx, rx)]
        level = "layout-tested" if key in LAYOUT_CLAIMS else "tested"
        writer.writerow((EDITION, kind, key, name, section, team,
                         level if tx else "unsupported", level if rx else "unsupported",
                         *evidence, binding, ""))
    return out.getvalue()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("pdf", type=Path)
    parser.add_argument("--write", action="store_true", help="replace the catalog after reconciliation")
    args = parser.parse_args()
    if hashlib.sha256(args.pdf.read_bytes()).hexdigest() != DIGEST:
        parser.error("unexpected source digest; review the edition before updating the inventory")
    text = subprocess.check_output(["pdftotext", "-layout", str(args.pdf.resolve()), "-"],
                                   encoding="utf-8")
    expected = render(text)
    if args.write:
        CATALOG.write_text(expected, encoding="utf-8")
    elif CATALOG.read_text(encoding="utf-8") != expected:
        parser.error("release inventory differs from the reconciled chapter")
    print(f"Verified {len(expected.splitlines()) - 1} inventory entries for {EDITION}")


if __name__ == "__main__":
    main()
