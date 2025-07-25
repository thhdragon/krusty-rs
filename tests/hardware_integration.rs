// Unit and integration tests for hardware modules and integration points

use krusty_rs::hardware::temperature::{TemperatureController, TemperatureStatus};
use krusty_rs::hardware::hardware_traits::TemperatureControllerTrait;
use krusty_rs::hardware::FanController;
use krusty_rs::hardware::GenericSensor;
use krusty_rs::hardware::hardware_traits::PeripheralTrait;

#[test]
fn test_temperature_controller_trait() {
    let mut temp = TemperatureController::new(22.2, 1.08, 114.0);
    temp.set_target_temperature(200.0);
    assert_eq!(temp.get_current_temperature(), 0.0);
    temp.update(1.0).unwrap();
    assert!(temp.get_current_temperature() > 0.0);
}

#[test]
fn test_fan_controller_peripheral_trait() {
    let mut fan = FanController::new();
    assert!(fan.perform_action("set_speed").is_ok());
    assert!(fan.perform_action("invalid_action").is_err());
}

#[test]
fn test_generic_sensor_peripheral_trait() {
    let mut sensor = GenericSensor::new();
    sensor.set_value(42.0);
    assert!(sensor.perform_action("read").is_ok());
    assert!(sensor.perform_action("invalid_action").is_err());
}

// Add more integration tests for async channel communication and subsystem integration as needed.
