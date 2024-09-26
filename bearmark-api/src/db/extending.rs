use diesel::{
    expression::AsExpression,
    sql_types::{Text, VarChar},
    Expression,
};

diesel::infix_operator!(RegexMatch, " ~ ", backend: diesel::pg::Pg);

pub trait RegexMatchExtensions: Expression<SqlType = VarChar> + Sized {
    fn regex_match<T: AsExpression<Text>>(self, other: T) -> RegexMatch<Self, T::Expression> {
        RegexMatch::new(self, other.as_expression())
    }
}

impl<T: Expression<SqlType = VarChar>> RegexMatchExtensions for T {}
