use crate::cli::Command;
use crate::constants::{flags, masks::*};

pub type MaskT = u8;

impl From<&Command> for MaskT {
    fn from(value: &Command) -> Self {
        match value {
            Command::PairAndTrust => PAIR,
            Command::Power { .. } => POWER,
            Command::ColorRgb { .. } => COLOR_RGB,
            Command::ColorHex { .. } => COLOR_HEX,
            Command::ColorXy { .. } => COLOR_XY,
            Command::Brightness { .. } => BRIGHTNESS,
            Command::Disconnect => DISCONNECT,
        }
    }
}

pub fn get_commands_from_flags(flags: MaskT) -> Vec<Command> {
    use flags::*;

    let mut v = Vec::new();

    if (flags >> (PAIR - 1)) & 1 == 1 {
        v.push(Command::PairAndTrust);
    }
    if (flags >> (POWER - 1)) & 1 == 1 {
        v.push(Command::Power { state: None })
    }
    if (flags >> (COLOR_RGB - 1)) & 1 == 1 {
        v.push(Command::ColorRgb {
            r: None,
            g: None,
            b: None,
        })
    }
    if (flags >> (COLOR_HEX - 1)) & 1 == 1 {
        v.push(Command::ColorHex { hex: None })
    }
    if (flags >> (COLOR_XY - 1)) & 1 == 1 {
        v.push(Command::ColorXy { x: None, y: None })
    }
    if (flags >> (BRIGHTNESS - 1)) & 1 == 1 {
        v.push(Command::Brightness { value: None })
    }
    if (flags >> (DISCONNECT - 1)) & 1 == 1 {
        v.push(Command::Disconnect)
    }

    v
}
