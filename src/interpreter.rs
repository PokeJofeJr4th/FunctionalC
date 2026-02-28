use std::{collections::HashMap, rc::Rc};

use crate::{
    interpreter::representation::{IRExpr, LValue},
    parser::syntax::Expression,
};

pub mod representation;

pub fn interpret(syn: Expression) -> Result<IRExpr, String> {
    to_ir(syn, HashMap::new())
}

type Context = HashMap<Rc<str>, LValue>;

fn to_ir(syn: Expression, mut context: Context) -> Result<IRExpr, String> {
    match syn {
        Expression::String(s) => Ok(IRExpr::String(s)),
        Expression::Ident(i) => match context.get(&i) {
            Some(&lvalue) => Ok(IRExpr::GetLocal(lvalue)),
            None => Err(format!("Unresolved identifier: `{i}`")),
        },
        Expression::Int(i) => Ok(IRExpr::Int(i)),
        Expression::Float(f) => Ok(IRExpr::Float(f)),
        Expression::BinaryOperation(lhs, op, rhs) => Ok(IRExpr::BinaryOperation(
            Box::new(to_ir(*lhs, context.clone())?),
            op,
            Box::new(to_ir(*rhs, context)?),
        )),
        Expression::If {
            condition,
            body,
            else_body,
        } => Ok(IRExpr::If {
            condition: Box::new(to_ir(*condition, context.clone())?),
            body: Box::new(to_ir(*body, context.clone())?),
            else_body: Box::new(to_ir(*else_body, context)?),
        }),
        Expression::Let { var, val, body } => {
            let new_lvalue = LValue::new();
            let val_ir = to_ir(*val, context.clone())?;
            context.insert(var, new_lvalue);
            let body_ir = to_ir(*body, context)?;
            Ok(IRExpr::SetLocal(
                new_lvalue,
                Box::new(val_ir),
                Box::new(body_ir),
            ))
        }
        Expression::FunctionCall { function, args } => todo!(),
    }
}
