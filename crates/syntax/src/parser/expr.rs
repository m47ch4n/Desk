use chumsky::prelude::*;
use uuid::Uuid;

use crate::{lexer::Token, span::Spanned};

use super::{
    common::{
        concat_range, parse_collection, parse_effectful, parse_function, parse_let_in, parse_op,
        parse_typed, ParserExt,
    },
    r#type::{self, Type},
};

#[derive(Clone, Debug, PartialEq)]
pub enum Literal {
    String(String),
    Int(i64),
    Rational(i64, i64),
    Float(f64),
}

#[derive(Clone, Debug, PartialEq)]
pub struct Handler {
    pub ty: Spanned<Type>,
    pub expr: Spanned<Expr>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Expr {
    Literal(Literal),
    Let {
        definition: Box<Spanned<Self>>,
        expression: Box<Spanned<Self>>,
    },
    Perform {
        effect: Box<Spanned<Self>>,
    },
    Effectful {
        class: Spanned<Type>,
        expr: Box<Spanned<Self>>,
        handlers: Vec<Handler>,
    },
    Call {
        function: Spanned<Type>,
        arguments: Vec<Spanned<Self>>,
    },
    Product(Vec<Spanned<Self>>),
    Typed {
        ty: Spanned<Type>,
        expr: Box<Spanned<Self>>,
    },
    Hole,
    Function {
        parameters: Vec<Spanned<Type>>,
        body: Box<Spanned<Self>>,
    },
    Array(Vec<Spanned<Self>>),
    Set(Vec<Spanned<Self>>),
    Module(Box<Spanned<Self>>),
    Import {
        ty: Spanned<Type>,
        uuid: Option<Uuid>,
    },
    Export {
        ty: Spanned<Type>,
        uuid: Option<Uuid>,
    },
}

pub fn parser() -> impl Parser<Token, Spanned<Expr>, Error = Simple<Token>> + Clone {
    recursive(|expr| {
        let hole = just(Token::Hole).to(Expr::Hole);
        let int64 = filter_map(|span, token| match token {
            Token::Int(int) => Ok(int),
            _ => Err(Simple::custom(span, "expected int literal")),
        });
        let rational = int64
            .then_ignore(just(Token::Divide))
            .then(int64)
            .map(|(a, b)| Expr::Literal(Literal::Rational(a, b)));
        let literal = rational
            .or(int64.map(|int| Expr::Literal(Literal::Int(int))))
            .or(filter_map(|span, token| match token {
                Token::Str(string) => Ok(Expr::Literal(Literal::String(string))),
                _ => Err(Simple::custom(span, "expected string literal")),
            }));
        let type_ = r#type::parser().delimited_by(Token::TypeBegin, Token::TypeEnd);
        let let_in =
            parse_let_in(expr.clone(), type_.clone()).map(|(definition, type_, expression)| {
                Expr::Let {
                    definition: Box::new(if let Some(type_) = type_ {
                        let span = concat_range(&definition.1, &type_.1);
                        (
                            Expr::Typed {
                                ty: type_,
                                expr: Box::new(definition),
                            },
                            span,
                        )
                    } else {
                        definition
                    }),
                    expression: Box::new(expression),
                }
            });
        let perform = just(Token::Perform)
            .ignore_then(expr.clone())
            .map(|effect| Expr::Perform {
                effect: Box::new(effect),
            });
        let effectful =
            parse_effectful(expr.clone(), type_.clone()).map(|(class, expr, handlers)| {
                Expr::Effectful {
                    class,
                    expr: Box::new(expr),
                    handlers: handlers
                        .into_iter()
                        .map(|handler| Handler {
                            ty: handler.0,
                            expr: handler.1,
                        })
                        .collect(),
                }
            });
        let call = type_
            .clone()
            .then(expr.clone().separated_by(just(Token::Comma)))
            .map(|(function, arguments)| Expr::Call {
                function,
                arguments,
            })
            .dot();
        let product =
            parse_op(just(Token::Product), expr.clone()).map(|values| Expr::Product(values));
        let function = parse_function(
            just(Token::Lambda),
            type_.clone(),
            just(Token::Arrow),
            expr.clone(),
        )
        .map(|(parameters, body)| Expr::Function {
            parameters,
            body: Box::new(body),
        });
        let array =
            parse_collection(Token::ArrayBegin, expr.clone(), Token::ArrayEnd).map(Expr::Array);
        let set = parse_collection(Token::SetBegin, expr.clone(), Token::SetEnd).map(Expr::Set);
        let typed = parse_typed(expr.clone(), type_.clone()).map(|(expr, ty)| Expr::Typed {
            ty,
            expr: Box::new(expr),
        });
        hole.or(literal)
            .or(let_in)
            .or(perform)
            .or(effectful)
            .or(call)
            .or(product)
            .or(function)
            .or(array)
            .or(set)
            .or(typed)
            .map_with_span(|token, span| (token, span))
    })
}

#[cfg(test)]
mod tests {
    use matches::assert_matches;

    use chumsky::Stream;

    use crate::lexer::lexer;

    use super::*;

    fn parse(input: &str) -> Result<Spanned<Expr>, Vec<Simple<Token>>> {
        parser().parse(Stream::from_iter(
            input.len()..input.len() + 1,
            dbg!(lexer().then_ignore(end()).parse(input).unwrap().into_iter()),
        ))
    }

    #[test]
    fn parse_literal_int() {
        assert_eq!(parse("-12").unwrap().0, Expr::Literal(Literal::Int(-12)));
    }

    #[test]
    fn parse_literal_rational() {
        assert_eq!(
            parse("1/2").unwrap().0,
            Expr::Literal(Literal::Rational(1, 2))
        );
    }

    #[test]
    fn parse_literal_string() {
        assert_eq!(
            parse(r#""abc""#).unwrap().0,
            Expr::Literal(Literal::String("abc".into()))
        );
    }

    #[test]
    fn parse_let() {
        assert_eq!(
            parse("$ 3: <'number> ~ ?.").unwrap().0,
            Expr::Let {
                definition: Box::new((
                    Expr::Typed {
                        expr: Box::new((Expr::Literal(Literal::Int(3)), 2..3)),
                        ty: (Type::Number, 6..13)
                    },
                    2..13
                )),
                expression: Box::new((Expr::Hole, 17..18)),
            }
        );
    }

    #[test]
    fn parse_let_without_type_in_and_dot() {
        assert_matches!(
            parse("$ 3 ?").unwrap().0,
            Expr::Let {
                definition: box (Expr::Literal(Literal::Int(3)), _),
                expression: box (Expr::Hole, _),
            }
        );
    }

    #[test]
    fn parse_perform() {
        assert_matches!(
            parse("! ?").unwrap().0,
            Expr::Perform {
                effect: box (Expr::Hole, _),
            }
        );
    }

    #[test]
    fn parse_handle() {
        let trait_ = parse(r#"# <'a class> ?; <'a num_to_num> => ?, <'a str_to_str> => ?"#)
            .unwrap()
            .0;
        assert_eq!(
            trait_,
            Expr::Effectful {
                class: (Type::Alias("class".into()), 3..11),
                expr: Box::new((Expr::Hole, 13..14)),
                handlers: vec![
                    Handler {
                        ty: (Type::Alias("num_to_num".into()), 17..30),
                        expr: (Expr::Hole, 35..36),
                    },
                    Handler {
                        ty: (Type::Alias("str_to_str".into()), 39..52),
                        expr: (Expr::Hole, 57..58),
                    }
                ]
            }
        );
    }

    #[test]
    fn parse_call() {
        assert_eq!(
            parse("<'a add> 1, 2.").unwrap().0,
            Expr::Call {
                function: (Type::Alias("add".into()), 1..7),
                arguments: vec![
                    (Expr::Literal(Literal::Int(1)), 9..10),
                    (Expr::Literal(Literal::Int(2)), 12..13)
                ],
            }
        );
    }

    #[test]
    fn parse_product() {
        assert_eq!(
            parse("* 1, ?").unwrap().0,
            Expr::Product(vec![
                (Expr::Literal(Literal::Int(1)), 2..3),
                (Expr::Hole, 5..6),
            ])
        );
    }

    #[test]
    fn parse_function() {
        assert_eq!(
            parse(r#"\ <'number>, <'number> -> ?"#).unwrap().0,
            Expr::Function {
                parameters: vec![(Type::Number, 3..10), (Type::Number, 14..21)],
                body: Box::new((Expr::Hole, 26..27)),
            }
        );
    }

    #[test]
    fn parse_array() {
        assert_eq!(
            parse("[1, ?, ?]").unwrap().0,
            Expr::Array(vec![
                (Expr::Literal(Literal::Int(1)), 1..2),
                (Expr::Hole, 4..5),
                (Expr::Hole, 7..8),
            ])
        );
    }

    #[test]
    fn parse_set() {
        assert_eq!(
            parse("{1, ?, ?}").unwrap().0,
            Expr::Set(vec![
                (Expr::Literal(Literal::Int(1)), 1..2),
                (Expr::Hole, 4..5),
                (Expr::Hole, 7..8),
            ])
        );
    }

    #[test]
    fn parse_type_annotation() {
        assert_eq!(
            parse("^?: <'number>").unwrap().0,
            Expr::Typed {
                expr: Box::new((Expr::Hole, 1..2)),
                ty: (Type::Number, 5..12),
            }
        );
    }
}
