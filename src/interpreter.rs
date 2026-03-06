use std::{
    collections::{HashMap, HashSet},
    rc::Rc,
};

use crate::{
    interpreter::representation::{
        ArithmeticOperator, BooleanOperator, Builtin, ComparisonOperator, IRExpr, IRType, IRValue,
        LValue,
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
        Expression::Ident(i) => interpret_identifier(&context, &i),
        Expression::Int(i) => Ok(IRExpr::Int(i).typed(IRType::Int)),
        Expression::Float(f) => Ok(IRExpr::Float(f).typed(IRType::Float)),
        Expression::BinaryOperation(lhs, op, rhs) => binary_op_to_ir(context, *lhs, op, *rhs),
        Expression::Ternary {
            condition,
            body,
            else_body,
        } => interpret_ternary(context, *condition, *body, *else_body),
        Expression::Let { var, val, body } => {
            let new_lvalue = LValue::new();
            let val_ir = to_ir(*val, context.clone())?;
            context.insert(var, (new_lvalue, val_ir.type_hint().clone()));
            let body_ir = to_ir(*body, context)?;
            let ty = body_ir.type_hint().clone();
            Ok(IRExpr::SetLocal(new_lvalue, Box::new(val_ir), Box::new(body_ir)).typed(ty))
        }
        Expression::MonadLet { var, val, body } => interpret_monad_let(context, var, *val, *body),
        Expression::FunctionCall { function, args } => {
            let args = args
                .into_iter()
                .map(|arg| to_ir(arg, context.clone()))
                .collect::<Result<Vec<IRValue>, String>>()?;
            let func = to_ir(*function, context)?;
            let IRType::Function { inputs, output } = func.type_hint() else {
                return Err(format!("Expected a function; got `{:?}`", func.type_hint()));
            };
            if inputs.len() != args.len() {
                return Err(format!(
                    "Wrong number of arguments; expected `{}` but got `{}`",
                    inputs.len(),
                    args.len()
                ));
            }
            for (arg, param_type) in args.iter().zip(inputs.iter()) {
                if param_type != arg.type_hint() {
                    return Err(format!(
                        "Function type parameter mismatch; expected `{param_type:?}` but got `{:?}`",
                        arg.type_hint()
                    ));
                }
            }
            let output = (**output).clone();
            Ok(IRExpr::FunctionCall(Box::new(func), args).typed(output))
        }
        Expression::Function { args, body } => {
            let mut new_context = context.clone();
            let mut params = Vec::new();
            let mut inputs = Vec::new();
            for (v, t) in args {
                let new_lv = LValue::new();
                params.push(new_lv);
                inputs.push(t.clone());
                new_context.insert(v, (new_lv, t));
            }
            let body = to_ir(*body, new_context)?;
            let out_ty = body.type_hint().clone();
            let captures = find_captures(&body, &params);
            Ok(IRExpr::Function {
                captures,
                params,
                body: Box::new(body),
            }
            .typed(IRType::Function {
                inputs,
                output: Box::new(out_ty),
            }))
        }
        Expression::ComposeMonads(first, second) => {
            let first = to_ir(*first, context.clone())?;
            let second = to_ir(*second, context)?;
            let (IRType::IOMonad(None), IRType::IOMonad(None)) =
                (first.type_hint(), second.type_hint())
            else {
                return Err(format!(
                    "Monad composition expects two `IO<void>`; got `{:?}` and `{:?}`",
                    first.type_hint(),
                    second.type_hint()
                ));
            };
            Ok(IRExpr::ComposeMonads(Box::new(first), Box::new(second))
                .typed(IRType::IOMonad(None)))
        }
    }
}

fn interpret_monad_let(
    mut context: HashMap<Rc<str>, (LValue, IRType)>,
    var: Rc<str>,
    val: Expression,
    body: Expression,
) -> Result<IRValue, String> {
    let new_lvalue = LValue::new();
    let val_ir = to_ir(val, context.clone())?;
    let IRType::IOMonad(retty) = val_ir.type_hint() else {
        return Err(format!(
            "Expected `IO<...>`; got `{:?}`",
            val_ir.type_hint()
        ));
    };
    let Some(retty) = retty else {
        return Err("Can't extract value from `IO<void>`".to_string());
    };
    context.insert(var, (new_lvalue, (**retty).clone()));
    let body_ir = to_ir(body, context)?;
    let ty = body_ir.type_hint().clone();
    if !ty.is_io_monad() {
        return Err(format!("Expected `IO<...>`; got `{ty:?}`"));
    }
    let captures = find_captures(&body_ir, &[]);
    Ok(IRExpr::BindIoMonad {
        var_name: new_lvalue,
        var_value: Box::new(val_ir),
        body: Box::new(body_ir),
        captures,
    }
    .typed(ty))
}

fn interpret_identifier(
    context: &HashMap<Rc<str>, (LValue, IRType)>,
    i: &str,
) -> Result<IRValue, String> {
    match (context.get(i), i) {
        (Some((lvalue, ty)), _) => Ok(IRExpr::GetLocal(*lvalue).typed(ty.clone())),
        (None, "return") => Ok(IRExpr::Builtin(Builtin::Return).typed(IRType::Function {
            inputs: vec![IRType::Int],
            output: Box::new(IRType::IOMonad(Some(Box::new(IRType::Int)))),
        })),
        (None, "readLine") => Ok(IRExpr::Builtin(Builtin::ReadLine)
            .typed(IRType::IOMonad(Some(Box::new(IRType::String))))),
        (None, "writeLine") => Ok(IRExpr::Builtin(Builtin::WriteLine).typed(IRType::Function {
            inputs: vec![IRType::String],
            output: Box::new(IRType::IOMonad(None)),
        })),
        (None, _) => Err(format!("Unresolved identifier: `{i}`")),
    }
}

fn interpret_ternary(
    context: HashMap<Rc<str>, (LValue, IRType)>,
    condition: Expression,
    body: Expression,
    else_body: Expression,
) -> Result<IRValue, String> {
    let condition = to_ir(condition, context.clone())?;
    let body = to_ir(body, context.clone())?;
    let else_body = to_ir(else_body, context)?;
    if condition.type_hint() != &IRType::Boolean {
        return Err(format!(
            "Expected boolean for ternary condition; got `{:?}`",
            condition.type_hint()
        ));
    }
    if body.type_hint() != else_body.type_hint() {
        return Err(format!(
            "Options in a ternary must be same type; got `{:?}` and `{:?}`",
            body.type_hint(),
            else_body.type_hint()
        ));
    }
    let ty = body.type_hint().clone();
    Ok(IRExpr::If {
        condition: Box::new(condition),
        body: Box::new(body),
        else_body: Box::new(else_body),
    }
    .typed(ty))
}

fn binary_op_to_ir(
    context: HashMap<Rc<str>, (LValue, IRType)>,
    lhs: Expression,
    op: BinaryOperator,
    rhs: Expression,
) -> Result<IRValue, String> {
    let lhs = to_ir(lhs, context.clone())?;
    let rhs = to_ir(rhs, context)?;
    match (op, lhs.type_hint(), rhs.type_hint()) {
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
            Ok(IRExpr::Arithmetic(
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
            )
            .typed(ty))
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
        ) if lt == rt => Ok(IRExpr::Comparison(
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
        )
        .typed(IRType::Boolean)),
        (op @ (BinaryOperator::And | BinaryOperator::Or), IRType::Boolean, IRType::Boolean) => {
            Ok(IRExpr::Boolean(
                Box::new(lhs),
                match op {
                    BinaryOperator::And => BooleanOperator::And,
                    BinaryOperator::Or => BooleanOperator::Or,
                    _ => unreachable!(),
                },
                Box::new(rhs),
            )
            .typed(IRType::Boolean))
        }
        (t, l, r) => Err(format!(
            "Invalid types for operation `{t:?}`: `{l:?}` and `{r:?}`"
        )),
    }
}

fn find_captures(body: &IRValue, params: &[LValue]) -> Vec<(LValue, IRType)> {
    fn visit_captures(
        val: &IRValue,
        values: &mut HashMap<LValue, IRType>,
        blacklist: &mut HashSet<LValue>,
    ) {
        match val.expr() {
            IRExpr::GetLocal(lvalue) => {
                values.insert(*lvalue, val.type_hint().clone());
            }
            IRExpr::SetLocal(lvalue, irvalue, irvalue1) => {
                blacklist.insert(*lvalue);
                visit_captures(irvalue, values, blacklist);
                visit_captures(irvalue1, values, blacklist);
            }
            IRExpr::Int(_) | IRExpr::Float(_) | IRExpr::String(_) | IRExpr::Builtin(_) => (),
            IRExpr::If {
                condition,
                body,
                else_body,
            } => {
                visit_captures(condition, values, blacklist);
                visit_captures(body, values, blacklist);
                visit_captures(else_body, values, blacklist);
            }
            IRExpr::Arithmetic(irvalue, _, irvalue1)
            | IRExpr::Comparison(irvalue, _, irvalue1)
            | IRExpr::Boolean(irvalue, _, irvalue1) => {
                visit_captures(irvalue, values, blacklist);
                visit_captures(irvalue1, values, blacklist);
            }
            IRExpr::Function {
                params,
                captures,
                body,
            } => {
                blacklist.extend(params);
                values.extend(captures.iter().cloned());
                visit_captures(body, values, blacklist);
            }
            IRExpr::BindIoMonad {
                var_name,
                var_value,
                body,
                captures,
            } => {
                blacklist.insert(*var_name);
                values.extend(captures.iter().cloned());
                visit_captures(var_value, values, blacklist);
                visit_captures(body, values, blacklist);
            }
            IRExpr::FunctionCall(irvalue, irvalues) => {
                visit_captures(irvalue, values, blacklist);
                for i in irvalues {
                    visit_captures(i, values, blacklist);
                }
            }
            IRExpr::ComposeMonads(first, second) => {
                visit_captures(first, values, blacklist);
                visit_captures(second, values, blacklist);
            }
        }
    }
    let mut values = HashMap::new();
    let mut blacklist = params.iter().copied().collect();
    visit_captures(body, &mut values, &mut blacklist);
    values
        .into_iter()
        .filter(|(k, _)| !blacklist.contains(k))
        .collect()
}
