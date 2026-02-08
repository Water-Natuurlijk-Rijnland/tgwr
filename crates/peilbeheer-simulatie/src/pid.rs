/// PID-regelaar voor gemaal sturing.
///
/// Regelt het debiet op basis van de afwijking van het streefpeil.
/// Kp=5.0, Ki=0.05, Kd=20.0 zijn de standaardwaarden uit de Python implementatie.
pub struct PidController {
    kp: f64,
    ki: f64,
    kd: f64,
    integral: f64,
    last_error: f64,
}

impl PidController {
    pub fn new(kp: f64, ki: f64, kd: f64) -> Self {
        Self {
            kp,
            ki,
            kd,
            integral: 0.0,
            last_error: 0.0,
        }
    }

    /// Bereken PID output (0.0 - 1.0) op basis van error en tijdstap.
    ///
    /// `error`: huidige_waterstand - streefpeil
    /// `dt`: tijdstap in minuten
    ///
    /// Returns: output factor (0.0 = pomp uit, 1.0 = vol vermogen)
    pub fn update(&mut self, error: f64, dt: f64) -> f64 {
        if dt == 0.0 {
            return 0.0;
        }

        // Integral alleen bijwerken bij significante error
        if error.abs() > 0.001 {
            self.integral += error * dt;
        }

        let derivative = (error - self.last_error) / dt;
        let output = (self.kp * error) + (self.ki * self.integral) + (self.kd * derivative);

        // Clamp naar 0.0 - 1.0
        let output = output.clamp(0.0, 1.0);

        // Anti-windup: corrigeer integral als output gesatureerd is
        if (output == 0.0 && error < 0.0) || (output == 1.0 && error > 0.0) {
            self.integral -= error * dt;
        }

        self.last_error = error;
        output
    }

    pub fn reset(&mut self) {
        self.integral = 0.0;
        self.last_error = 0.0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pid_zero_error() {
        let mut pid = PidController::new(5.0, 0.05, 20.0);
        let output = pid.update(0.0, 1.0);
        assert!((output - 0.0).abs() < 0.001);
    }

    #[test]
    fn test_pid_positive_error() {
        let mut pid = PidController::new(5.0, 0.05, 20.0);
        // Water is boven streefpeil → pomp moet aan
        let output = pid.update(0.1, 1.0);
        assert!(output > 0.0);
    }

    #[test]
    fn test_pid_negative_error() {
        let mut pid = PidController::new(5.0, 0.05, 20.0);
        // Water is onder streefpeil → pomp uit
        let output = pid.update(-0.5, 1.0);
        assert!((output - 0.0).abs() < 0.001);
    }

    #[test]
    fn test_pid_output_clamped() {
        let mut pid = PidController::new(5.0, 0.05, 20.0);
        // Grote positieve error → output clamped op 1.0
        let output = pid.update(10.0, 1.0);
        assert!((output - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_pid_reset() {
        let mut pid = PidController::new(5.0, 0.05, 20.0);
        pid.update(0.5, 1.0);
        pid.reset();
        // After reset, same error should give same result as fresh
        let mut pid2 = PidController::new(5.0, 0.05, 20.0);
        let o1 = pid.update(0.1, 1.0);
        let o2 = pid2.update(0.1, 1.0);
        assert!((o1 - o2).abs() < 0.001);
    }
}
