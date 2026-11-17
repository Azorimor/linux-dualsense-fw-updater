mod common;

use common::{MockSonyHttp, NoopSleeper, SimDualSenseController, synthetic_firmware};
use dualsense_core::{
    BoardModel, CancellationToken, DEFAULT_PID, DUALSENSE_EDGE_PID, DualSenseUpdater,
    FirmwareCatalogClient, UpdateConfig, decode_ds5_color, resolve_controller_target,
};

#[test]
fn end_to_end_board_detection_serial_color_and_unsupported_handling() {
    // 1. Supported BDM-050 controller with 0x80/0x81 serial & colorway detection
    let mut sim_bdm050 = SimDualSenseController::new(0x0520);
    sim_bdm050.fw_series = 0x000B;
    sim_bdm050.hw_info = 0x0000_0811;
    sim_bdm050.serial_number = Some("F12302ABCDEF12345".to_string());

    let updater_050 = DualSenseUpdater::with_sleeper_and_config(
        sim_bdm050,
        NoopSleeper::default(),
        UpdateConfig::default(),
    );
    let info = updater_050.read_firmware_info().expect("firmware info");
    assert_eq!(info.build_date, "May 01 2025");
    assert_eq!(info.board_model(), BoardModel::Bdm050);
    assert_eq!(info.fw_series(), 0x000B);
    assert_eq!(info.hw_info(), 0x0000_0811);

    let hw = resolve_controller_target(&info, DEFAULT_PID);
    assert_eq!(hw.board_name, "BDM-050");
    assert_eq!(hw.supported_target_id, Some("000B"));
    assert!(hw.is_officially_supported);

    let serial = updater_050
        .try_read_serial_number()
        .expect("serial should be decoded");
    assert_eq!(serial, "F12302ABCDEF12345");
    assert_eq!(decode_ds5_color(&serial), Some("Cosmic Red"));

    // Resolve Sony catalog release using auto-detected target ID
    let raw_fw = synthetic_firmware(512, 0x0630);
    let catalog_client = FirmwareCatalogClient::new(MockSonyHttp {
        image_bytes: raw_fw,
    });
    let cancel = CancellationToken::new();
    let catalog = catalog_client.fetch_catalog(&cancel).expect("catalog");
    let release = catalog
        .release_for_target(hw.supported_target_id.unwrap())
        .expect("release");
    let image = catalog_client
        .download_firmware(release, &cancel, &mut |_| {})
        .expect("download");
    updater_050
        .run_full_update(&image, &cancel, |_| {})
        .expect("update succeeds");

    // 2. Unsupported BDM-010 where 0x80/0x81 errors cleanly without breaking read_firmware_info
    let mut sim_bdm010 = SimDualSenseController::new(0x0200);
    sim_bdm010.fw_series = 0x0001;
    sim_bdm010.hw_info = 0x0000_0300;
    sim_bdm010.serial_number = None;

    let updater_010 = DualSenseUpdater::with_sleeper_and_config(
        sim_bdm010,
        NoopSleeper::default(),
        UpdateConfig::default(),
    );
    let info_010 = updater_010
        .read_firmware_info()
        .expect("0x20 succeeds even if 0x80/0x81 unsupported");
    assert_eq!(updater_010.try_read_serial_number(), None);

    let hw_010 = resolve_controller_target(&info_010, DEFAULT_PID);
    assert_eq!(hw_010.board_name, "BDM-010");
    assert_eq!(hw_010.supported_target_id, None);
    assert!(!hw_010.is_officially_supported);
    assert!(hw_010.unsupported_reason.is_some());

    // 3. DualSense Edge resolves to target 0044
    let mut sim_edge = SimDualSenseController::new(0x0217);
    sim_edge.fw_series = 0x0044;
    sim_edge.hw_info = 0x0000_0400;
    let updater_edge = DualSenseUpdater::new(sim_edge);
    let info_edge = updater_edge.read_firmware_info().expect("edge info");
    let hw_edge = resolve_controller_target(&info_edge, DUALSENSE_EDGE_PID);
    assert_eq!(hw_edge.board_name, "DualSense Edge");
    assert_eq!(hw_edge.supported_target_id, Some("0044"));
    assert!(hw_edge.is_officially_supported);
}
