use std::sync::LazyLock;

use color_space::Rgb;

use log::*;

// Limits for Hue Play lights
// https://developers.meethue.com/develop/application-design-guidance/color-conversion-formulas-rgb-to-xy-and-back/#Gamut
static RED: LazyLock<Xy> = LazyLock::new(|| Xy::new(0.6915, 0.3038));
static GREEN: LazyLock<Xy> = LazyLock::new(|| Xy::new(0.17, 0.7));
static BLUE: LazyLock<Xy> = LazyLock::new(|| Xy::new(0.1532, 0.0475));

#[derive(Debug, Clone, Copy)]
pub struct Xy {
    pub x: f64,
    pub y: f64,
    pub brightness: Option<f64>,
}

impl PartialEq for Xy {
    fn eq(&self, other: &Self) -> bool {
        (self.x - other.x).abs() < f64::EPSILON && (self.y - other.y).abs() < f64::EPSILON
    }
}

impl Xy {
    pub fn new(x: f64, y: f64) -> Self {
        Self {
            x,
            y,
            brightness: None,
        }
    }

    // https://developers.meethue.com/develop/application-design-guidance/color-conversion-formulas-rgb-to-xy-and-back/#xy-to-rgb-color
    pub fn to_rgb(mut self, brightness: f64) -> Rgb {
        if !self.is_within_color_gamut() {
            self = self.closest_point_in_triangle(&RED, &GREEN, &BLUE);
        }

        // To XYZ
        let y = brightness;
        let x = (y / self.y) * self.x;
        let z = (y / self.y) * (1. - self.x - self.y);

        self.brightness = Some(brightness);

        // To RGB using Wide RGB D65
        let mut r = x * 1.656492 - y * 0.354851 - z * 0.255038;
        let mut g = -x * 0.707196 + y * 1.655397 + z * 0.036152;
        let mut b = x * 0.051713 - y * 0.121364 + z * 1.011530;

        // Clamp values to valid range
        r = r.clamp(0.0, 1.0);
        g = g.clamp(0.0, 1.0);
        b = b.clamp(0.0, 1.0);

        // Gamma correction
        r = if r <= 0.0031308 {
            12.92 * r
        } else {
            (1.0 + 0.055) * r.powf(1.0 / 2.4) - 0.055
        };
        g = if g <= 0.0031308 {
            12.92 * g
        } else {
            (1.0 + 0.055) * g.powf(1.0 / 2.4) - 0.055
        };
        b = if b <= 0.0031308 {
            12.92 * b
        } else {
            (1.0 + 0.055) * b.powf(1.0 / 2.4) - 0.055
        };

        debug!("values after calc {:?} {:?}", self, (r, g, b));

        Rgb::new(r * 255., g * 255., b * 255.)
    }

    pub fn is_within_color_gamut(&self) -> bool {
        let (x, y) = (self.x, self.y);
        let (x1, y1) = (RED.x, RED.y);
        let (x2, y2) = (GREEN.x, GREEN.y);
        let (x3, y3) = (BLUE.x, BLUE.y);

        let denominator = (y2 - y3) * (x1 - x3) + (x3 - x2) * (y1 - y3);

        let lambda1 = ((y2 - y3) * (x - x3) + (x3 - x2) * (y - y3)) / denominator;
        let lambda2 = ((y3 - y1) * (x - x3) + (x1 - x3) * (y - y3)) / denominator;
        let lambda3 = 1. - lambda1 - lambda2;

        (0.0..=1.0).contains(&lambda1)
            && (0.0..=1.0).contains(&lambda2)
            && (0.0..=1.0).contains(&lambda3)
    }

    fn closest_point_in_triangle(&self, x1: &Self, x2: &Self, x3: &Self) -> Self {
        let euclidean_distance =
            |a: &Self, b: &Self| ((a.x - b.x).powi(2) + (a.y - b.y).powi(2)).sqrt();

        let p1_closest = self.project_point_to_line_segment(x1, x2);
        let p2_closest = self.project_point_to_line_segment(x2, x3);
        let p3_closest = self.project_point_to_line_segment(x3, x1);

        let d1 = euclidean_distance(&p1_closest, self);
        let d2 = euclidean_distance(&p2_closest, self);
        let d3 = euclidean_distance(&p3_closest, self);

        if d1 < d2 && d1 < d3 {
            p1_closest
        } else if d2 < d1 && d2 < d3 {
            p2_closest
        } else {
            p3_closest
        }
    }

    fn project_point_to_line_segment(&self, a: &Self, b: &Self) -> Self {
        let ab = Self {
            x: b.x - a.x,
            y: b.y - a.y,
            brightness: None,
        };
        let ap = Self {
            x: self.x - a.x,
            y: self.y - a.y,
            brightness: None,
        };

        let t = ((ap.x * ab.x + ap.y * ab.y) / (ab.x * ab.x + ab.y * ab.y)).clamp(0., 1.);

        Self {
            x: a.x + t * ab.x,
            y: a.y + t * ab.y,
            brightness: None,
        }
    }
}

impl From<Rgb> for Xy {
    fn from(rgb: Rgb) -> Self {
        let (r, g, b) = (rgb.r / 255., rgb.g / 255., rgb.b / 255.);

        // Gamma correction
        let red = if r > 0.04045 {
            ((r + 0.055) / (1.0 + 0.055)).powf(2.4)
        } else {
            r / 12.92
        };
        let green = if g > 0.04045 {
            ((g + 0.055) / (1.0 + 0.055)).powf(2.4)
        } else {
            g / 12.92
        };
        let blue = if b > 0.04045 {
            ((b + 0.055) / (1.0 + 0.055)).powf(2.4)
        } else {
            b / 12.92
        };

        // RGB to XYZ Wide RGB D65 conversion formula
        let x = red * 0.4124 + green * 0.3576 + blue * 0.1805;
        let y = red * 0.2126 + green * 0.7152 + blue * 0.0722;
        let z = red * 0.0193 + green * 0.1192 + blue * 0.9505;

        // Xy from XYZ
        let brightness = y;
        let x = x / (x + y + z);
        let y = y / (x + y + z);

        let xy = Self {
            x,
            y,
            brightness: Some(brightness),
        };

        if !xy.is_within_color_gamut() {
            return xy.closest_point_in_triangle(&RED, &GREEN, &BLUE);
        }

        xy
    }
}

#[cfg(test)]
mod color_tests {
    use super::*;

    fn xy_from_rgb() {
        let red_rgb = Rgb::new(255., 0., 0.);
        let red_xy = Xy::from(red_rgb);

        assert_eq!(red_xy, *RED);
    }

    fn xy_rgb_consistency() {
        let red = Rgb::new(255., 0., 0.);
        let xy = Xy::from(red);
        let rgb = xy.to_rgb(1.);

        debug!("{:?} {:?} {:?}", red, xy, rgb);

        assert_eq!(red.r, rgb.r, "Red R isn't equal to RGB R");
        assert_eq!(red.g, rgb.g, "Red G isn't equal to RGB G");
        assert_eq!(red.b, rgb.b, "Red B isn't equal to RGB B");
    }
}
