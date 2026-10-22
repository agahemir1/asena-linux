//! Pure, bounded search ranking and calculator. No filesystem or GTK access.
use crate::{apps::DesktopEntry, text::fold};

pub fn rank(query: &str, name: &str) -> u32 {
    let query = fold(query);
    let name = fold(name);
    if query.is_empty() {
        return 0;
    }
    if name == query {
        return 500;
    }
    if name.starts_with(&query) {
        return 400;
    }
    if name.split_whitespace().any(|word| word.starts_with(&query)) {
        return 300;
    }
    if name.contains(&query) {
        return 200;
    }
    let mut letters = query.chars();
    let mut next = letters.next();
    for letter in name.chars() {
        if next == Some(letter) {
            next = letters.next();
        }
    }
    if next.is_none() {
        100
    } else {
        0
    }
}

pub fn app_score(query: &str, app: &DesktopEntry, launches: u32) -> u32 {
    let score = rank(query, &app.name).max(
        app.generic_name
            .iter()
            .chain(app.keywords.iter())
            .map(|field| rank(query, field).saturating_sub(20))
            .max()
            .unwrap_or(0),
    );
    // Usage cannot move a weaker matching class above a stronger one.
    if score == 0 {
        0
    } else {
        score + (launches.saturating_add(1).ilog2()).min(30)
    }
}

pub fn calculate(input: &str) -> Option<f64> {
    if input.len() > 256 {
        return None;
    }
    let mut parser = Calculator {
        input: input.as_bytes(),
        position: 0,
        depth: 0,
    };
    let result = parser.expression(0)?;
    parser.space();
    (parser.position == parser.input.len() && result.is_finite()).then_some(result)
}

struct Calculator<'a> {
    input: &'a [u8],
    position: usize,
    depth: u8,
}
impl Calculator<'_> {
    fn space(&mut self) {
        while self
            .input
            .get(self.position)
            .is_some_and(u8::is_ascii_whitespace)
        {
            self.position += 1;
        }
    }
    fn expression(&mut self, minimum: u8) -> Option<f64> {
        self.space();
        self.depth += 1;
        if self.depth > 32 {
            return None;
        }
        let mut left = match self.input.get(self.position)? {
            b'+' => {
                self.position += 1;
                self.expression(5)?
            }
            b'-' => {
                self.position += 1;
                -self.expression(5)?
            }
            b'(' => {
                self.position += 1;
                let value = self.expression(0)?;
                self.space();
                if self.input.get(self.position) != Some(&b')') {
                    return None;
                }
                self.position += 1;
                value
            }
            _ => {
                let start = self.position;
                while self
                    .input
                    .get(self.position)
                    .is_some_and(|c| c.is_ascii_digit() || *c == b'.')
                {
                    self.position += 1;
                }
                std::str::from_utf8(&self.input[start..self.position])
                    .ok()?
                    .parse()
                    .ok()?
            }
        };
        loop {
            self.space();
            let Some(&operator) = self.input.get(self.position) else {
                break;
            };
            let (low, high) = match operator {
                b'+' | b'-' => (1, 2),
                b'*' | b'/' | b'%' => (3, 4),
                _ => break,
            };
            if low < minimum {
                break;
            }
            self.position += 1;
            let right = self.expression(high)?;
            left = match operator {
                b'+' => left + right,
                b'-' => left - right,
                b'*' => left * right,
                b'/' => left / right,
                _ => left % right,
            };
        }
        self.depth -= 1;
        Some(left)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn turkish_and_match_classes() {
        assert_eq!(rank("ISIK", "Işık"), 500);
        assert_eq!(rank("AYAR", "Ayarlar"), rank("ayar", "Ayarlar"));
        let names = ["ayar", "ayarlar", "Sistem ayarları", "kayar", "aygıtlar"];
        let scores: Vec<_> = names.iter().map(|s| rank("ayar", s)).collect();
        assert!(scores.windows(2).all(|p| p[0] > p[1]));
        assert_eq!(rank("xyz", "ayar"), 0);
    }
    #[test]
    fn calculator_precedence_invalid_and_bounded_input() {
        assert_eq!(calculate("2+2*3"), Some(8.0));
        assert_eq!(calculate("-(2+3) / 2"), Some(-2.5));
        for input in ["1/0", "2;touch x", "(2", "2 3", "NaN", "", "2..3", "2+"] {
            assert_eq!(calculate(input), None, "{input}");
        }
        assert_eq!(calculate(&"(".repeat(256)), None);
    }
}
