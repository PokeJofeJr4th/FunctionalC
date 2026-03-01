use std::{collections::HashMap, rc::Rc};

use crate::{
    interpreter::representation::{
        ArithmeticOperator, BooleanOperator, ComparisonOperator, IRExpr, IRType, IRValue, LValue,
    },
    parser::syntax::{BinaryOperator, Expression},
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
            match (op, &lhs.1, &rhs.1) {
                (
                    op @ (BinaryOperator::Add
                    | BinaryOperator::Sub
                    | BinaryOperator::Mul
                    | BinaryOperator::Div
                    | BinaryOperator::Mod),
                    ty @ IRType::Int,
                    IRType::Int,
                )
                | (
                    op @ (BinaryOperator::Add
                    | BinaryOperator::Sub
                    | BinaryOperator::Mul
                    | BinaryOperator::Div
                    | BinaryOperator::Mod),
                    ty @ IRType::Float,
                    IRType::Float,
                ) => {
                    let ty = ty.clone();
                    Ok(IRValue(
                        IRExpr::Arithmetic(
                            Box::new(lhs),
                            match op {
                                BinaryOperator::Add => ArithmeticOperator::Add,
                                BinaryOperator::Sub => ArithmeticOperator::Sub,
                                BinaryOperator::Mul => ArithmeticOperator::Mul,
                                BinaryOperator::Div => ArithmeticOperator::Div,
                                BinaryOperator::Mod => ArithmeticOperator::Mod,
                                _ => unreachable!(),
                            },
                            Box::new(rhs),
                        ),
                        ty,
                    ))
                }
                (
                    op @ (BinaryOperator::Eq
                    | BinaryOperator::Ne
                    | BinaryOperator::Lt
                    | BinaryOperator::Le
                    | BinaryOperator::Gt
                    | BinaryOperator::Ge),
                    lt,
                    rt,
                ) if lt == rt => Ok(IRValue(
                    IRExpr::Comparison(
                        Box::new(lhs),
                        match op {
                            BinaryOperator::Eq => ComparisonOperator::Eq,
                            BinaryOperator::Ne => ComparisonOperator::Ne,
                            BinaryOperator::Lt => ComparisonOperator::Lt,
                            BinaryOperator::Le => ComparisonOperator::Le,
                            BinaryOperator::Gt => ComparisonOperator::Gt,
                            BinaryOperator::Ge => ComparisonOperator::Ge,
                            _ => unreachable!(),
                        },
                        Box::new(rhs),
                    ),
                    IRType::Boolean,
                )),
                (
                    op @ (BinaryOperator::And | BinaryOperator::Or),
                    IRType::Boolean,
                    IRType::Boolean,
                ) => Ok(IRValue(
                    IRExpr::Boolean(
                        Box::new(lhs),
                        match op {
                            BinaryOperator::And => BooleanOperator::And,
                            BinaryOperator::Or => BooleanOperator::Or,
                            _ => unreachable!(),
                        },
                        Box::new(rhs),
                    ),
                    IRType::Boolean,
                )),
                (t, l, r) => Err(format!(
                    "Invalid types for operation `{t:?}`: `{l:?}` and `{r:?}`"
                )),
            }
        }
        Expression::If {
            condition,
            body,
            else_body,
        } => {
            let condition = to_ir(*condition, context.clone())?;
            let body = to_ir(*body, context.clone())?;
            let else_body = to_ir(*else_body, context)?;
            if condition.1 != IRType::Boolean {
                return Err(format!(
                    "Expected boolean for ternary condition; got `{:?}`",
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
        Expression::FunctionCall { function, args } => {
            let args = args
                .into_iter()
                .map(|arg| to_ir(arg, context.clone()))
                .collect::<Result<Vec<IRValue>, String>>()?;
            let func = to_ir(*function, context)?;
            Err(format!("Expected a function; got `{func:?}`"))
        }
    }
}
