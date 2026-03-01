use std::{collections::HashMap, rc::Rc};

use crate::{
    interpreter::representation::{IRExpr, IRType, IRValue, LValue},
    parser::syntax::Expression,
};

pub mod representation;

pub fn interpret(syn: Expression) -> Result<IRValue, String> {
    to_ir(syn, HashMap::new())
}

type Context = HashMap<Rc<str>, (LValue, IRType)>;

fn to_ir(syn: Expression, mut context: Context) -> Result<IRValue, String> {
    match syn {
        Expression::String(s) => Ok(IRExpr::String(s).typed(IRType::String)),
        Expression::Ident(i) => match context.get(&i) {
            Some((lvalue, ty)) => Ok(IRExpr::GetLocal(*lvalue).typed(ty.clone())),
            None => Err(format!("Unresolved identifier: `{i}`")),
        },
        Expression::Int(i) => Ok(IRExpr::Int(i).typed(IRType::Int)),
        Expression::Float(f) => Ok(IRExpr::Float(f).typed(IRType::Float)),
        Expression::BinaryOperation(lhs, op, rhs) => {
            let lhs = to_ir(*lhs, context.clone())?;
            let rhs = to_ir(*rhs, context)?;
            let ty = match (op, &lhs.1, &rhs.1) {
                (_, IRType::Int, IRType::Int) => IRType::Int,
                (t, l, r) => {
                    return Err(format!(
                        "Invalid types for operation `{t:?}`: `{l:?}` and `{r:?}`; expected two numeric types"
                    ));
                }
            };
            Ok(IRExpr::BinaryOperation(Box::new(lhs), op, Box::new(rhs)).typed(ty))
        }
        Expression::If {
            condition,
            body,
            else_body,
        } => {
            let condition = to_ir(*condition, context.clone())?;
            let body = to_ir(*body, context.clone())?;
            let else_body = to_ir(*else_body, context)?;
            if condition.1 != IRType::Int {
                return Err(format!(
                    "Expected int for ternary condition; got `{:?}`",
                    condition.1
                ));
            }
            if body.1 != else_body.1 {
                return Err(format!(
                    "Options in a ternary must be same type; got `{:?}` and `{:?}`",
                    body.1, else_body.1
                ));
            }
            let ty = body.1.clone();
            Ok(IRExpr::If {
                condition: Box::new(condition),
                body: Box::new(body),
                else_body: Box::new(else_body),
            }
            .typed(ty))
        }
        Expression::Let { var, val, body } => {
            let new_lvalue = LValue::new();
            let val_ir = to_ir(*val, context.clone())?;
            context.insert(var, (new_lvalue, val_ir.1.clone()));
            let body_ir = to_ir(*body, context)?;
            let ty = val_ir.1.clone();
            Ok(IRExpr::SetLocal(new_lvalue, Box::new(val_ir), Box::new(body_ir)).typed(ty))
        }
        Expression::FunctionCall { function, args } => todo!(),
    }
}
