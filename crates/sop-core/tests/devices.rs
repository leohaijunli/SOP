//! The `devices` setting, checked against the shared vectors.
//!
//! The same `devices-vectors.json` drives the TypeScript mirror (`ui/src/lib/devices.ts`,
//! run by `npm run test:devices`), so the two parsers cannot drift apart unnoticed.

use serde_json::Value;
use sop_core::settings::{DeviceStock, Settings, format_devices, parse_devices};

fn vectors() -> Value {
    serde_json::from_str(include_str!("devices-vectors.json")).unwrap()
}

#[test]
fn the_devices_setting_matches_the_shared_vectors() {
    let vectors = vectors();
    for case in vectors["parse"].as_array().unwrap() {
        let input = case["input"].as_str().unwrap();
        let expected: Vec<DeviceStock> = case["devices"]
            .as_array()
            .unwrap()
            .iter()
            .map(|device| DeviceStock {
                kind: device["kind"].as_str().unwrap().to_owned(),
                name: device["name"].as_str().unwrap().to_owned(),
                serials: device["serials"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|serial| serial.as_str().unwrap().to_owned())
                    .collect(),
            })
            .collect();
        assert_eq!(parse_devices(input), expected, "parse input: {input}");
    }
    for case in vectors["normalise"].as_array().unwrap() {
        let input = case["input"].as_str().unwrap();
        let output = case["output"].as_str().unwrap();
        assert_eq!(
            format_devices(&parse_devices(input)),
            output,
            "normalise input: {input}"
        );
    }
}

#[test]
fn the_devices_setting_round_trips_through_the_settings() {
    let mut settings = Settings::default();
    assert_eq!(
        settings.get("devices").as_deref(),
        Some(""),
        "no devices by default"
    );

    settings
        .set(
            "devices",
            "GNSS receiver/Trimble R10: 123 ; UAV ; Base station/NetRS: 9001,9002",
        )
        .unwrap();
    assert_eq!(
        settings.get("devices").as_deref(),
        Some("GNSS receiver/Trimble R10: 123; UAV; Base station/NetRS: 9001, 9002")
    );

    // Emptying the box means "no other devices", not an error.
    settings.set("devices", "   ").unwrap();
    assert_eq!(settings.get("devices").as_deref(), Some(""));

    settings.set("devices", "UAV").unwrap();
    settings.unset("devices").unwrap();
    assert_eq!(settings.get("devices").as_deref(), Some(""));
}

#[test]
fn a_device_name_or_serial_with_a_separator_is_rejected() {
    let mut settings = Settings::default();
    // A serial with a colon would be re-parsed as a second device, so it is refused.
    let error = settings
        .set("devices", "UAV/serial: 12:34")
        .unwrap_err()
        .to_string();
    assert!(error.contains("devices"), "{error}");
    assert_eq!(
        settings.get("devices").as_deref(),
        Some(""),
        "a rejected value is not stored"
    );

    // A name cannot smuggle in a slash.
    let error = settings
        .set("devices", "UAV/a/b: 1")
        .unwrap_err()
        .to_string();
    assert!(error.contains("devices"), "{error}");
    assert_eq!(settings.get("devices").as_deref(), Some(""));
}
