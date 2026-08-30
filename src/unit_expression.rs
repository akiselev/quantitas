use crate::{Dimension, ExactScale, UnitExpressionError, UnitRegistry};

/// A parsed and evaluated compound unit expression, such as `W/(m*K)`.
///
/// `rendering` is a canonical, fully-parenthesized textual form: two
/// expressions with the same rendering are guaranteed to denote the same
/// dimension and scale, and different renderings reflect a genuine
/// precedence or grouping difference (e.g. `(a/b)*c` versus `a/(b*c)`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UnitExpression {
    pub dimension: Dimension,
    pub scale: ExactScale,
    pub rendering: String,
}

impl UnitRegistry {
    /// Parses and evaluates a compound unit expression against this registry.
    ///
    /// Grammar (whitespace-insensitive):
    ///
    /// ```text
    /// expr     := term (('*' | '/') term)*
    /// term     := primary ('^' exponent)?
    /// primary  := IDENT | '(' expr ')'
    /// exponent := '-'? DIGITS
    /// ```
    ///
    /// `IDENT` resolves against [`UnitRegistry::by_symbol`]; affine units
    /// (those with an `offset_to_si`) are refused inside a compound
    /// expression, since a scale-only composition cannot carry an offset.
    pub fn parse_unit_expression(
        &self,
        input: &str,
    ) -> Result<UnitExpression, UnitExpressionError> {
        let tokens = tokenize(input)?;
        if tokens.is_empty() {
            return Err(UnitExpressionError::Empty);
        }
        let mut parser = Parser {
            tokens: &tokens,
            pos: 0,
        };
        let ast = parser.parse_expr()?;
        if parser.pos != tokens.len() {
            return Err(UnitExpressionError::UnexpectedToken {
                offset: tokens[parser.pos].offset,
            });
        }
        let (dimension, scale) = evaluate(self, &ast)?;
        Ok(UnitExpression {
            dimension,
            scale,
            rendering: render(&ast),
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum Ast {
    Ident(String, usize),
    Pow(Box<Ast>, i32),
    Mul(Box<Ast>, Box<Ast>),
    Div(Box<Ast>, Box<Ast>),
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum TokenKind {
    Ident(String),
    Number(String),
    Star,
    Slash,
    Caret,
    Minus,
    LParen,
    RParen,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Token {
    kind: TokenKind,
    offset: usize,
}

const MIN_EXPONENT: i64 = -12;
const MAX_EXPONENT: i64 = 12;

fn tokenize(input: &str) -> Result<Vec<Token>, UnitExpressionError> {
    let mut tokens = Vec::new();
    let mut chars = input.char_indices().peekable();
    while let Some(&(offset, character)) = chars.peek() {
        if character.is_whitespace() {
            chars.next();
            continue;
        }
        match character {
            '*' => {
                tokens.push(Token {
                    kind: TokenKind::Star,
                    offset,
                });
                chars.next();
            }
            '/' => {
                tokens.push(Token {
                    kind: TokenKind::Slash,
                    offset,
                });
                chars.next();
            }
            '^' => {
                tokens.push(Token {
                    kind: TokenKind::Caret,
                    offset,
                });
                chars.next();
            }
            '-' => {
                tokens.push(Token {
                    kind: TokenKind::Minus,
                    offset,
                });
                chars.next();
            }
            '(' => {
                tokens.push(Token {
                    kind: TokenKind::LParen,
                    offset,
                });
                chars.next();
            }
            ')' => {
                tokens.push(Token {
                    kind: TokenKind::RParen,
                    offset,
                });
                chars.next();
            }
            c if c.is_ascii_digit() => {
                let mut text = String::new();
                while let Some(&(_, digit)) = chars.peek() {
                    if digit.is_ascii_digit() {
                        text.push(digit);
                        chars.next();
                    } else {
                        break;
                    }
                }
                tokens.push(Token {
                    kind: TokenKind::Number(text),
                    offset,
                });
            }
            c if c.is_alphabetic() => {
                let mut text = String::new();
                while let Some(&(_, letter)) = chars.peek() {
                    if letter.is_alphabetic() {
                        text.push(letter);
                        chars.next();
                    } else {
                        break;
                    }
                }
                tokens.push(Token {
                    kind: TokenKind::Ident(text),
                    offset,
                });
            }
            other => {
                return Err(UnitExpressionError::UnexpectedCharacter {
                    character: other,
                    offset,
                });
            }
        }
    }
    Ok(tokens)
}

struct Parser<'a> {
    tokens: &'a [Token],
    pos: usize,
}

impl<'a> Parser<'a> {
    fn peek(&self) -> Option<&'a Token> {
        self.tokens.get(self.pos)
    }

    fn advance(&mut self) -> Option<&'a Token> {
        let token = self.tokens.get(self.pos);
        if token.is_some() {
            self.pos += 1;
        }
        token
    }

    fn parse_expr(&mut self) -> Result<Ast, UnitExpressionError> {
        let mut node = self.parse_term()?;
        loop {
            match self.peek().map(|token| &token.kind) {
                Some(TokenKind::Star) => {
                    self.advance();
                    let rhs = self.parse_term()?;
                    node = Ast::Mul(Box::new(node), Box::new(rhs));
                }
                Some(TokenKind::Slash) => {
                    self.advance();
                    let rhs = self.parse_term()?;
                    node = Ast::Div(Box::new(node), Box::new(rhs));
                }
                _ => break,
            }
        }
        Ok(node)
    }

    fn parse_term(&mut self) -> Result<Ast, UnitExpressionError> {
        let primary = self.parse_primary()?;
        if let Some(TokenKind::Caret) = self.peek().map(|token| &token.kind) {
            self.advance();
            let exponent = self.parse_exponent()?;
            Ok(Ast::Pow(Box::new(primary), exponent))
        } else {
            Ok(primary)
        }
    }

    fn parse_primary(&mut self) -> Result<Ast, UnitExpressionError> {
        match self.peek() {
            Some(Token {
                kind: TokenKind::Ident(name),
                offset,
            }) => {
                let node = Ast::Ident(name.clone(), *offset);
                self.advance();
                Ok(node)
            }
            Some(Token {
                kind: TokenKind::LParen,
                offset,
            }) => {
                let open_offset = *offset;
                self.advance();
                let inner = self.parse_expr()?;
                match self.peek() {
                    Some(Token {
                        kind: TokenKind::RParen,
                        ..
                    }) => {
                        self.advance();
                        Ok(inner)
                    }
                    _ => Err(UnitExpressionError::UnclosedParenthesis {
                        offset: open_offset,
                    }),
                }
            }
            Some(token) => Err(UnitExpressionError::UnexpectedToken {
                offset: token.offset,
            }),
            None => Err(UnitExpressionError::UnexpectedEnd),
        }
    }

    fn parse_exponent(&mut self) -> Result<i32, UnitExpressionError> {
        let negative = matches!(self.peek().map(|token| &token.kind), Some(TokenKind::Minus));
        if negative {
            self.advance();
        }
        match self.peek() {
            Some(Token {
                kind: TokenKind::Number(digits),
                offset,
            }) => {
                let offset = *offset;
                let digits = digits.clone();
                self.advance();
                let magnitude: i64 = digits
                    .parse()
                    .map_err(|_| UnitExpressionError::ExponentOutOfRange { offset })?;
                let value = if negative { -magnitude } else { magnitude };
                if !(MIN_EXPONENT..=MAX_EXPONENT).contains(&value) {
                    return Err(UnitExpressionError::ExponentOutOfRange { offset });
                }
                Ok(value as i32)
            }
            Some(token) => Err(UnitExpressionError::UnexpectedToken {
                offset: token.offset,
            }),
            None => Err(UnitExpressionError::UnexpectedEnd),
        }
    }
}

fn evaluate(
    registry: &UnitRegistry,
    ast: &Ast,
) -> Result<(Dimension, ExactScale), UnitExpressionError> {
    match ast {
        Ast::Ident(name, offset) => {
            let unit =
                registry
                    .by_symbol(name)
                    .ok_or_else(|| UnitExpressionError::UnknownSymbol {
                        symbol: name.clone(),
                        offset: *offset,
                    })?;
            if unit.offset_to_si.is_some() {
                return Err(UnitExpressionError::AffineUnit {
                    symbol: name.clone(),
                });
            }
            Ok((unit.dimension, unit.scale_to_si))
        }
        Ast::Mul(left, right) => {
            let (left_dimension, left_scale) = evaluate(registry, left)?;
            let (right_dimension, right_scale) = evaluate(registry, right)?;
            Ok((
                left_dimension.checked_product(right_dimension)?,
                left_scale.checked_mul(right_scale)?,
            ))
        }
        Ast::Div(left, right) => {
            let (left_dimension, left_scale) = evaluate(registry, left)?;
            let (right_dimension, right_scale) = evaluate(registry, right)?;
            Ok((
                left_dimension.checked_quotient(right_dimension)?,
                left_scale.checked_div(right_scale)?,
            ))
        }
        Ast::Pow(base, exponent) => {
            let (base_dimension, base_scale) = evaluate(registry, base)?;
            Ok((
                base_dimension.checked_powi(*exponent)?,
                base_scale.checked_powi(*exponent)?,
            ))
        }
    }
}

fn render(ast: &Ast) -> String {
    match ast {
        Ast::Ident(name, _) => name.clone(),
        Ast::Pow(base, exponent) => format!("{}^{exponent}", render_atom(base)),
        Ast::Mul(left, right) => format!("{}*{}", render_operand(left), render_operand(right)),
        Ast::Div(left, right) => format!("{}/{}", render_operand(left), render_operand(right)),
    }
}

/// Renders the operand of a `^`, parenthesizing anything but a bare identifier.
fn render_atom(node: &Ast) -> String {
    match node {
        Ast::Ident(..) => render(node),
        _ => format!("({})", render(node)),
    }
}

/// Renders the operand of a `*`/`/`, parenthesizing nested `*`/`/` so the
/// rendering is unambiguous regardless of the original grouping.
fn render_operand(node: &Ast) -> String {
    match node {
        Ast::Mul(..) | Ast::Div(..) => format!("({})", render(node)),
        _ => render(node),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{RationalExponent, UnitId};

    /// `[Mass, Length, Time, ElectricCurrent, Temperature, Amount, LuminousIntensity]`.
    fn dim(values: [i32; 7]) -> Dimension {
        Dimension([
            RationalExponent::integer(values[0]),
            RationalExponent::integer(values[1]),
            RationalExponent::integer(values[2]),
            RationalExponent::integer(values[3]),
            RationalExponent::integer(values[4]),
            RationalExponent::integer(values[5]),
            RationalExponent::integer(values[6]),
        ])
    }

    fn scale(numerator: i128, denominator: i128, power10: i32) -> ExactScale {
        ExactScale::new(numerator, denominator, power10).unwrap()
    }

    #[test]
    fn golden_expressions_resolve_dimension_and_scale() {
        let registry = UnitRegistry::si_bootstrap();
        let cases: &[(&str, [i32; 7], ExactScale)] = &[
            ("W/(m*K)", [1, 1, -3, 0, -1, 0, 0], ExactScale::ONE),
            ("m^2/s", [0, 2, -1, 0, 0, 0, 0], ExactScale::ONE),
            ("mol/m^3", [0, -3, 0, 0, 0, 1, 0], ExactScale::ONE),
            ("mol/(m^2*s)", [0, -2, -1, 0, 0, 1, 0], ExactScale::ONE),
            ("N/m^3", [1, -2, -2, 0, 0, 0, 0], ExactScale::ONE),
            ("Pa", [1, -1, -2, 0, 0, 0, 0], ExactScale::ONE),
            ("kg/(m*s)", [1, -1, -1, 0, 0, 0, 0], ExactScale::ONE),
            ("m/s", [0, 1, -1, 0, 0, 0, 0], ExactScale::ONE),
            ("m/s^2", [0, 1, -2, 0, 0, 0, 0], ExactScale::ONE),
            ("kg/m^3", [1, -3, 0, 0, 0, 0, 0], ExactScale::ONE),
            ("J/(kg*K)", [0, 2, -2, 0, -1, 0, 0], ExactScale::ONE),
            ("W/m^3", [1, -1, -3, 0, 0, 0, 0], ExactScale::ONE),
            ("A/m^2", [0, -2, 0, 1, 0, 0, 0], ExactScale::ONE),
            ("C/m^3", [0, -3, 1, 1, 0, 0, 0], ExactScale::ONE),
            ("S/m", [-1, -3, 3, 2, 0, 0, 0], ExactScale::ONE),
            ("F/m", [-1, -3, 4, 2, 0, 0, 0], ExactScale::ONE),
            ("kPa", [1, -1, -2, 0, 0, 0, 0], scale(1, 1, 3)),
            ("mm", [0, 1, 0, 0, 0, 0, 0], scale(1, 1, -3)),
            ("kmol/m^3", [0, -3, 0, 0, 0, 1, 0], scale(1, 1, 3)),
            ("s^-1", [0, 0, -1, 0, 0, 0, 0], ExactScale::ONE),
        ];
        for (expression, expected_dimension, expected_scale) in cases {
            let parsed = registry
                .parse_unit_expression(expression)
                .unwrap_or_else(|error| panic!("`{expression}` failed to parse: {error}"));
            assert_eq!(
                parsed.dimension,
                dim(*expected_dimension),
                "dimension mismatch for `{expression}`"
            );
            assert_eq!(
                parsed.scale, *expected_scale,
                "scale mismatch for `{expression}`"
            );
        }
    }

    #[test]
    fn precedence_distinguishes_left_assoc_chaining_from_explicit_grouping() {
        let registry = UnitRegistry::si_bootstrap();
        // (m/s)*kg: M^1 L^1 T^-1
        let chained = registry.parse_unit_expression("m/s*kg").unwrap();
        assert_eq!(chained.dimension, dim([1, 1, -1, 0, 0, 0, 0]));
        assert_eq!(chained.rendering, "(m/s)*kg");

        // m/(s*kg): M^-1 L^1 T^-1
        let grouped = registry.parse_unit_expression("m/(s*kg)").unwrap();
        assert_eq!(grouped.dimension, dim([-1, 1, -1, 0, 0, 0, 0]));
        assert_eq!(grouped.rendering, "m/(s*kg)");

        assert_ne!(chained.dimension, grouped.dimension);
        assert_ne!(chained.rendering, grouped.rendering);
    }

    #[test]
    fn whitespace_is_insignificant() {
        let registry = UnitRegistry::si_bootstrap();
        let tight = registry.parse_unit_expression("W/(m*K)").unwrap();
        let spaced = registry.parse_unit_expression(" W / ( m * K ) ").unwrap();
        assert_eq!(tight, spaced);
    }

    #[test]
    fn micro_prefix_accepts_ascii_and_unicode_spelling() {
        let registry = UnitRegistry::si_bootstrap();
        let ascii = registry.parse_unit_expression("uF").unwrap();
        let unicode = registry.parse_unit_expression("\u{b5}F").unwrap();
        assert_eq!(ascii.dimension, unicode.dimension);
        assert_eq!(ascii.scale, unicode.scale);
        assert_eq!(ascii.scale, scale(1, 1, -6));
    }

    #[test]
    fn unknown_symbol_is_refused() {
        let registry = UnitRegistry::si_bootstrap();
        let error = registry.parse_unit_expression("foo/m").unwrap_err();
        assert!(matches!(
            error,
            UnitExpressionError::UnknownSymbol { symbol, .. } if symbol == "foo"
        ));
    }

    #[test]
    fn affine_units_are_refused_inside_a_compound_expression() {
        let registry = UnitRegistry::si_bootstrap();
        let error = registry.parse_unit_expression("degC/s").unwrap_err();
        assert!(matches!(
            error,
            UnitExpressionError::AffineUnit { symbol } if symbol == "degC"
        ));
    }

    #[test]
    fn malformed_input_is_refused_with_typed_errors() {
        let registry = UnitRegistry::si_bootstrap();
        assert!(matches!(
            registry.parse_unit_expression(""),
            Err(UnitExpressionError::Empty)
        ));
        assert!(matches!(
            registry.parse_unit_expression("W/(m*K"),
            Err(UnitExpressionError::UnclosedParenthesis { .. })
        ));
        assert!(matches!(
            registry.parse_unit_expression("*m"),
            Err(UnitExpressionError::UnexpectedToken { .. })
        ));
        assert!(matches!(
            registry.parse_unit_expression("m@s"),
            Err(UnitExpressionError::UnexpectedCharacter { character: '@', .. })
        ));
        assert!(matches!(
            registry.parse_unit_expression("m^99"),
            Err(UnitExpressionError::ExponentOutOfRange { .. })
        ));
    }

    #[test]
    fn exact_scale_checked_arithmetic_round_trips() {
        let kilo = scale(1, 1, 3);
        let milli = scale(1, 1, -3);
        assert_eq!(kilo.checked_mul(milli).unwrap(), ExactScale::ONE);
        assert_eq!(ExactScale::ONE.checked_div(kilo).unwrap(), milli);
        assert_eq!(scale(2, 1, 0).checked_powi(3).unwrap(), scale(8, 1, 0));
        assert_eq!(scale(2, 1, 0).checked_powi(-1).unwrap(), scale(1, 2, 0));
        assert_eq!(scale(1, 1, -3).checked_powi(2).unwrap(), scale(1, 1, -6));
    }

    #[test]
    fn compound_literal_canonicalizes_through_registry_parse_and_canonicalize() {
        let registry = UnitRegistry::si_bootstrap();
        let (literal, display) = registry
            .parse(
                "2.5 W/(m*K)",
                crate::QuantityKindId::new("test:ThermalConductivity"),
            )
            .unwrap();
        assert_eq!(display.symbol, "W/(m*K)");
        let quantity = registry.canonicalize(&literal).unwrap();
        assert!((quantity.value_si() - 2.5).abs() < 1e-12);
        assert_eq!(quantity.dimension(), dim([1, 1, -3, 0, -1, 0, 0]));

        // Round trip: canonicalizing a literal built directly from the
        // canonical rendering (not via `parse`) must agree.
        let direct = crate::QuantityLiteral {
            value: 2.5,
            unit: UnitId::new("W/(m*K)"),
            kind: crate::QuantityKindId::new("test:ThermalConductivity"),
        };
        let direct_quantity = registry.canonicalize(&direct).unwrap();
        assert_eq!(direct_quantity, quantity);
    }
}
