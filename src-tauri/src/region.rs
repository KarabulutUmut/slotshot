use serde::Deserialize;

/// Fractions of the overlay viewport, independent of desktop origin and DPI.
#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Selection {
    pub x1: f64,
    pub y1: f64,
    pub x2: f64,
    pub y2: f64,
}

#[derive(Debug, PartialEq)]
pub struct PixelRegion {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

impl Selection {
    pub fn pixels(self, width: u32, height: u32) -> Result<PixelRegion, String> {
        if width == 0 || height == 0 {
            return Err("The display returned an empty image. Retry capture.".into());
        }
        if [self.x1, self.y1, self.x2, self.y2]
            .iter()
            .any(|v| !v.is_finite() || !(0.0..=1.0).contains(v))
        {
            return Err("Selection coordinates are outside the display.".into());
        }
        if self.x1 == self.x2 || self.y1 == self.y2 {
            return Err("The selection is empty. Drag a rectangle to capture.".into());
        }
        let x = (self.x1.min(self.x2) * f64::from(width)).floor() as u32;
        let y = (self.y1.min(self.y2) * f64::from(height)).floor() as u32;
        let right = (self.x1.max(self.x2) * f64::from(width)).ceil() as u32;
        let bottom = (self.y1.max(self.y2) * f64::from(height)).ceil() as u32;
        Ok(PixelRegion {
            x,
            y,
            width: right - x,
            height: bottom - y,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn selection(values: [f64; 4]) -> Selection {
        let [x1, y1, x2, y2] = values;
        Selection { x1, y1, x2, y2 }
    }

    #[test]
    fn retina_reverse_drag_uses_captured_pixels() {
        assert_eq!(
            selection([0.75, 0.5, 0.25, 0.1]).pixels(2880, 1800),
            Ok(PixelRegion {
                x: 720,
                y: 180,
                width: 1440,
                height: 720
            })
        );
    }

    #[test]
    fn fractional_scaling_rounds_outward_and_keeps_edges_in_bounds() {
        assert_eq!(
            selection([0.1, 0.1, 1.0, 1.0]).pixels(1366, 768),
            Ok(PixelRegion {
                x: 136,
                y: 76,
                width: 1230,
                height: 692
            })
        );
        // Monitor-local coordinates also apply to displays left/above primary.
        assert_eq!(
            selection([0.0, 0.0, 1.0, 1.0])
                .pixels(1920, 1080)
                .unwrap()
                .width,
            1920
        );
    }

    #[test]
    fn rejects_empty_invalid_and_nonfinite_selections() {
        for values in [
            [0.2, 0.2, 0.2, 0.5],
            [0.2, 0.2, 0.5, 0.2],
            [-0.1, 0.0, 0.5, 0.5],
            [0.0, 0.0, 1.1, 1.0],
            [f64::NAN, 0.0, 1.0, 1.0],
            [0.0, 0.0, f64::INFINITY, 1.0],
        ] {
            assert!(selection(values).pixels(100, 100).is_err());
        }
        assert!(selection([0.0, 0.0, 1.0, 1.0]).pixels(0, 100).is_err());
    }
}
