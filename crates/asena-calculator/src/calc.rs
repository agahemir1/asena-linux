use std::f64::consts::{E, PI};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CalcMode {
    Basic,
    Scientific,
    Programmer,
    Converter,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryOp {
    Add,
    Subtract,
    Multiply,
    Divide,
    Modulo,
    Power,
    BitAnd,
    BitOr,
    BitXor,
    ShiftLeft,
    ShiftRight,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOp {
    Negate,
    SquareRoot,
    Sin,
    Cos,
    Tan,
    Log10,
    Ln,
    Factorial,
    BitNot,
}

#[derive(Debug, Clone, PartialEq)]
pub struct HistoryEntry {
    pub expression: String,
    pub result: String,
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct CalculatorEngine {
    pub mode: CalcMode,
    pub current_input: String,
    pub previous_value: Option<f64>,
    pub pending_op: Option<BinaryOp>,
    pub memory: f64,
    pub history: Vec<HistoryEntry>,
    pub new_number: bool,
}

impl Default for CalculatorEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl CalculatorEngine {
    pub fn new() -> Self {
        Self {
            mode: CalcMode::Basic,
            current_input: "0".to_string(),
            previous_value: None,
            pending_op: None,
            memory: 0.0,
            history: Vec::new(),
            new_number: true,
        }
    }

    pub fn set_mode(&mut self, mode: CalcMode) {
        self.mode = mode;
    }

    pub fn input_digit(&mut self, ch: char) {
        if self.new_number || self.current_input == "0" {
            self.current_input = ch.to_string();
            self.new_number = false;
        } else {
            self.current_input.push(ch);
        }
    }

    pub fn input_dot(&mut self) {
        if self.new_number {
            self.current_input = "0.".to_string();
            self.new_number = false;
        } else if !self.current_input.contains('.') {
            self.current_input.push('.');
        }
    }

    pub fn backspace(&mut self) {
        if !self.new_number && self.current_input.len() > 1 {
            self.current_input.pop();
        } else {
            self.current_input = "0".to_string();
            self.new_number = true;
        }
    }

    pub fn clear(&mut self) {
        self.current_input = "0".to_string();
        self.previous_value = None;
        self.pending_op = None;
        self.new_number = true;
    }

    pub fn clear_entry(&mut self) {
        self.current_input = "0".to_string();
        self.new_number = true;
    }

    pub fn current_value(&self) -> f64 {
        self.current_input.parse::<f64>().unwrap_or(0.0)
    }

    pub fn set_current_value(&mut self, val: f64) {
        if val.is_nan() || val.is_infinite() {
            self.current_input = "Error".to_string();
        } else if (val.fract()).abs() < 1e-10 {
            self.current_input = format!("{:.0}", val);
        } else {
            self.current_input = format!("{:.8}", val)
                .trim_end_matches('0')
                .trim_end_matches('.')
                .to_string();
        }
        self.new_number = true;
    }

    pub fn set_binary_op(&mut self, op: BinaryOp) {
        let cur = self.current_value();
        if let Some(prev) = self.previous_value {
            if let Some(pending) = self.pending_op {
                let res = self.evaluate_op(prev, cur, pending);
                self.previous_value = Some(res);
                self.set_current_value(res);
            }
        } else {
            self.previous_value = Some(cur);
        }
        self.pending_op = Some(op);
        self.new_number = true;
    }

    pub fn apply_unary_op(&mut self, op: UnaryOp) {
        let val = self.current_value();
        let res = match op {
            UnaryOp::Negate => -val,
            UnaryOp::SquareRoot => {
                if val < 0.0 {
                    f64::NAN
                } else {
                    val.sqrt()
                }
            }
            UnaryOp::Sin => val.to_radians().sin(),
            UnaryOp::Cos => val.to_radians().cos(),
            UnaryOp::Tan => val.to_radians().tan(),
            UnaryOp::Log10 => {
                if val <= 0.0 {
                    f64::NAN
                } else {
                    val.log10()
                }
            }
            UnaryOp::Ln => {
                if val <= 0.0 {
                    f64::NAN
                } else {
                    val.ln()
                }
            }
            UnaryOp::Factorial => {
                if val < 0.0 || val.fract() != 0.0 || val > 170.0 {
                    f64::NAN
                } else {
                    let n = val as u64;
                    let mut acc = 1.0f64;
                    for i in 2..=n {
                        acc *= i as f64;
                    }
                    acc
                }
            }
            UnaryOp::BitNot => (!(val as i64)) as f64,
        };

        let op_name = match op {
            UnaryOp::Negate => "negate",
            UnaryOp::SquareRoot => "sqrt",
            UnaryOp::Sin => "sin",
            UnaryOp::Cos => "cos",
            UnaryOp::Tan => "tan",
            UnaryOp::Log10 => "log",
            UnaryOp::Ln => "ln",
            UnaryOp::Factorial => "fact",
            UnaryOp::BitNot => "not",
        };

        let expr = format!("{op_name}({val})");
        self.set_current_value(res);
        self.history.push(HistoryEntry {
            expression: expr,
            result: self.current_input.clone(),
        });
    }

    pub fn input_constant(&mut self, name: &str) {
        match name {
            "pi" => self.set_current_value(PI),
            "e" => self.set_current_value(E),
            _ => {}
        }
    }

    pub fn equals(&mut self) {
        if let (Some(prev), Some(op)) = (self.previous_value, self.pending_op) {
            let cur = self.current_value();
            let res = self.evaluate_op(prev, cur, op);

            let op_str = op_symbol(op);
            let expr = format!("{prev} {op_str} {cur}");
            self.set_current_value(res);
            self.history.push(HistoryEntry {
                expression: expr,
                result: self.current_input.clone(),
            });

            self.previous_value = None;
            self.pending_op = None;
            self.new_number = true;
        }
    }

    fn evaluate_op(&self, a: f64, b: f64, op: BinaryOp) -> f64 {
        match op {
            BinaryOp::Add => a + b,
            BinaryOp::Subtract => a - b,
            BinaryOp::Multiply => a * b,
            BinaryOp::Divide => {
                if b == 0.0 {
                    f64::NAN
                } else {
                    a / b
                }
            }
            BinaryOp::Modulo => {
                if b == 0.0 {
                    f64::NAN
                } else {
                    a % b
                }
            }
            BinaryOp::Power => a.powf(b),
            BinaryOp::BitAnd => ((a as i64) & (b as i64)) as f64,
            BinaryOp::BitOr => ((a as i64) | (b as i64)) as f64,
            BinaryOp::BitXor => ((a as i64) ^ (b as i64)) as f64,
            BinaryOp::ShiftLeft => ((a as i64) << (b as u32 % 64)) as f64,
            BinaryOp::ShiftRight => ((a as i64) >> (b as u32 % 64)) as f64,
        }
    }

    // Memory operations
    pub fn memory_clear(&mut self) {
        self.memory = 0.0;
    }

    pub fn memory_recall(&mut self) {
        self.set_current_value(self.memory);
    }

    pub fn memory_add(&mut self) {
        self.memory += self.current_value();
        self.new_number = true;
    }

    pub fn memory_subtract(&mut self) {
        self.memory -= self.current_value();
        self.new_number = true;
    }

    #[allow(dead_code)]
    pub fn memory_store(&mut self) {
        self.memory = self.current_value();
        self.new_number = true;
    }

    /// The left operand and operator waiting for the right operand.
    pub fn pending_expression(&self) -> String {
        match (self.previous_value, self.pending_op) {
            (Some(previous), Some(op)) => format!("{} {}", format_number(previous), op_symbol(op)),
            _ => String::new(),
        }
    }

    // Programmer base formatting
    pub fn programmer_representations(&self) -> ProgrammerBases {
        let val = self.current_value() as i64;
        ProgrammerBases {
            hex: format!("{:X}", val),
            dec: format!("{}", val),
            oct: format!("{:o}", val),
            bin: format!("{:b}", val),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProgrammerBases {
    pub hex: String,
    pub dec: String,
    pub oct: String,
    pub bin: String,
}

pub fn op_symbol(op: BinaryOp) -> &'static str {
    match op {
        BinaryOp::Add => "+",
        BinaryOp::Subtract => "−",
        BinaryOp::Multiply => "×",
        BinaryOp::Divide => "÷",
        BinaryOp::Modulo => "%",
        BinaryOp::Power => "^",
        BinaryOp::BitAnd => "AND",
        BinaryOp::BitOr => "OR",
        BinaryOp::BitXor => "XOR",
        BinaryOp::ShiftLeft => "<<",
        BinaryOp::ShiftRight => ">>",
    }
}

/// Compact display of a value, without a trailing ".0".
pub fn format_number(value: f64) -> String {
    if !value.is_finite() {
        return gettextrs::gettext("Error");
    }
    if value.fract() == 0.0 && value.abs() < 1e15 {
        format!("{value:.0}")
    } else {
        let text = format!("{value:.10}");
        text.trim_end_matches('0').trim_end_matches('.').to_owned()
    }
}

/// Unit names per category, in the index order used by `convert_units`.
pub fn unit_names(category: UnitCategory) -> Vec<String> {
    use gettextrs::gettext;
    match category {
        UnitCategory::Length => vec![
            gettext("Meters"),
            gettext("Kilometers"),
            gettext("Centimeters"),
            gettext("Millimeters"),
            gettext("Inches"),
            gettext("Feet"),
            gettext("Miles"),
        ],
        UnitCategory::Mass => vec![
            gettext("Kilograms"),
            gettext("Grams"),
            gettext("Milligrams"),
            gettext("Pounds"),
            gettext("Ounces"),
        ],
        UnitCategory::Temperature => {
            vec![gettext("Celsius"), gettext("Fahrenheit"), gettext("Kelvin")]
        }
        UnitCategory::Speed => vec![
            gettext("Kilometers per hour"),
            gettext("Meters per second"),
            gettext("Miles per hour"),
        ],
        UnitCategory::DataVolume => vec![
            gettext("Bytes"),
            gettext("Kibibytes"),
            gettext("Mebibytes"),
            gettext("Gibibytes"),
            gettext("Tebibytes"),
        ],
    }
}

// Unit conversion engine
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnitCategory {
    Length,
    Mass,
    Temperature,
    Speed,
    DataVolume,
}

pub fn convert_units(category: UnitCategory, value: f64, from_idx: usize, to_idx: usize) -> f64 {
    match category {
        UnitCategory::Length => {
            // [m, km, cm, mm, inch, ft, mile]
            let to_meter = [1.0, 1000.0, 0.01, 0.001, 0.0254, 0.3048, 1609.344];
            let from_factor = to_meter.get(from_idx).copied().unwrap_or(1.0);
            let to_factor = to_meter.get(to_idx).copied().unwrap_or(1.0);
            let in_meters = value * from_factor;
            in_meters / to_factor
        }
        UnitCategory::Mass => {
            // [kg, g, mg, lb, oz]
            let to_kg = [1.0, 0.001, 0.000001, 0.45359237, 0.02834952];
            let from_factor = to_kg.get(from_idx).copied().unwrap_or(1.0);
            let to_factor = to_kg.get(to_idx).copied().unwrap_or(1.0);
            let in_kg = value * from_factor;
            in_kg / to_factor
        }
        UnitCategory::Temperature => {
            // [Celsius, Fahrenheit, Kelvin]
            // Convert to Celsius first
            let in_c = match from_idx {
                0 => value,                      // Celsius
                1 => (value - 32.0) * 5.0 / 9.0, // Fahrenheit
                2 => value - 273.15,             // Kelvin
                _ => value,
            };
            // Convert Celsius to target
            match to_idx {
                0 => in_c,
                1 => (in_c * 9.0 / 5.0) + 32.0,
                2 => in_c + 273.15,
                _ => in_c,
            }
        }
        UnitCategory::Speed => {
            // [km/h, m/s, mph]
            let to_ms = [1.0 / 3.6, 1.0, 0.44704];
            let from_factor = to_ms.get(from_idx).copied().unwrap_or(1.0);
            let to_factor = to_ms.get(to_idx).copied().unwrap_or(1.0);
            let in_ms = value * from_factor;
            in_ms / to_factor
        }
        UnitCategory::DataVolume => {
            // [B, KB, MB, GB, TB]
            let to_b = [
                1.0,
                1024.0,
                1024.0 * 1024.0,
                1024.0 * 1024.0 * 1024.0,
                1024.0 * 1024.0 * 1024.0 * 1024.0,
            ];
            let from_factor = to_b.get(from_idx).copied().unwrap_or(1.0);
            let to_factor = to_b.get(to_idx).copied().unwrap_or(1.0);
            let in_b = value * from_factor;
            in_b / to_factor
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_helpers() {
        assert_eq!(format_number(12.0), "12");
        assert_eq!(format_number(0.125), "0.125");
        assert_eq!(format_number(f64::NAN), "Error");
        let mut calc = CalculatorEngine::new();
        calc.input_digit('8');
        calc.set_binary_op(BinaryOp::Divide);
        assert_eq!(calc.pending_expression(), "8 ÷");
        for category in [
            UnitCategory::Length,
            UnitCategory::Mass,
            UnitCategory::Temperature,
            UnitCategory::Speed,
            UnitCategory::DataVolume,
        ] {
            assert!(unit_names(category).len() >= 3);
        }
    }

    #[test]
    fn test_basic_arithmetic() {
        let mut calc = CalculatorEngine::new();
        calc.input_digit('5');
        calc.set_binary_op(BinaryOp::Add);
        calc.input_digit('7');
        calc.equals();
        assert_eq!(calc.current_input, "12");

        calc.set_binary_op(BinaryOp::Multiply);
        calc.input_digit('3');
        calc.equals();
        assert_eq!(calc.current_input, "36");
    }

    #[test]
    fn test_division_by_zero_safety() {
        let mut calc = CalculatorEngine::new();
        calc.input_digit('9');
        calc.set_binary_op(BinaryOp::Divide);
        calc.input_digit('0');
        calc.equals();
        assert_eq!(calc.current_input, "Error");
    }

    #[test]
    fn test_scientific_functions() {
        let mut calc = CalculatorEngine::new();
        calc.input_digit('1');
        calc.input_digit('6');
        calc.apply_unary_op(UnaryOp::SquareRoot);
        assert_eq!(calc.current_input, "4");

        calc.clear();
        calc.input_digit('5');
        calc.apply_unary_op(UnaryOp::Factorial);
        assert_eq!(calc.current_input, "120");
    }

    #[test]
    fn test_programmer_bases_and_bitwise() {
        let mut calc = CalculatorEngine::new();
        calc.input_digit('2');
        calc.input_digit('5');
        calc.input_digit('5');
        let bases = calc.programmer_representations();
        assert_eq!(bases.hex, "FF");
        assert_eq!(bases.dec, "255");
        assert_eq!(bases.oct, "377");
        assert_eq!(bases.bin, "11111111");

        calc.set_binary_op(BinaryOp::BitAnd);
        calc.input_digit('1');
        calc.input_digit('5');
        calc.equals();
        assert_eq!(calc.current_input, "15");
    }

    #[test]
    fn test_unit_conversions() {
        // 1 km to m = 1000 m
        let res = convert_units(UnitCategory::Length, 1.0, 1, 0);
        assert!((res - 1000.0).abs() < 1e-5);

        // 0 Celsius to Fahrenheit = 32 F
        let temp = convert_units(UnitCategory::Temperature, 0.0, 0, 1);
        assert!((temp - 32.0).abs() < 1e-5);
    }
}
