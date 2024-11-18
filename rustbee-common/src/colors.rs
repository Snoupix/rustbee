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
        self.x as u8 == other.x as u8 && self.y as u8 == other.y as u8
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

        // Weird checks stolen from
        // https://developers.meethue.com/develop/application-design-guidance/color-conversion-formulas-rgb-to-xy-and-back/#Color-rgb-to-xy
        if r > b && r > g && r > 1.0 {
            // red is too big
            g /= r;
            b /= r;
            r = 1.;
        } else if g > b && g > r && g > 1.0 {
            // green is too big
            r /= g;
            b /= g;
            g = 1.;
        } else if b > r && b > g && b > 1.0 {
            // blue is too big
            r /= b;
            g /= b;
            b = 1.;
        }

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

        // Weird checks stolen from
        // https://developers.meethue.com/develop/application-design-guidance/color-conversion-formulas-rgb-to-xy-and-back/#Color-rgb-to-xy
        if r > b && r > g {
            // red is biggest
            if r > 1. {
                g /= r;
                b /= r;
                r = 1.;
            }
        } else if g > b && g > r {
            // green is biggest
            if g > 1. {
                r /= g;
                b /= g;
                g = 1.;
            }
        } else if b > r && b > g {
            // blue is biggest
            if b > 1. {
                r /= b;
                g /= b;
                b = 1.;
            }
        }

        debug!("values after calc {:?} {:?}", self, (r, g, b));

        /*
        assert!(
            (0.0..1.).contains(&r),
            "After conversion, R should be between 0. && 1. Actual value {r}"
        );
        assert!(
            (0.0..1.).contains(&g),
            "After conversion, G should be between 0. && 1. Actual value {g}"
        );
        assert!(
            (0.0..1.).contains(&b),
            "After conversion, B should be between 0. && 1. Actual value {b}"
        );
        */

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

        (0. ..=1.).contains(&lambda1)
            && (0. ..=1.).contains(&lambda2)
            && (0. ..=1.).contains(&lambda3)
    }

    fn closest_point_in_triangle(&self, x1: &Self, x2: &Self, x3: &Self) -> Self {
        let euclidean_distance =
            |a: &Self, b: &Self| ((a.x - b.x).powi(2) + (a.y - b.y).powi(2)).powf(0.5);

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
        let (mut r, mut g, mut b) = (rgb.r, rgb.g, rgb.b);
        r /= 255.;
        g /= 255.;
        b /= 255.;

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

    // FIXME: It's not exactly 255 0 0 because of the color gamut
    fn xy_from_rgb() {
        let red_rgb = Rgb::new(255., 0., 0.);

        assert_eq!(red_rgb, RED.to_rgb(1.));
        assert_eq!(&*RED, &Xy::from(red_rgb));
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
