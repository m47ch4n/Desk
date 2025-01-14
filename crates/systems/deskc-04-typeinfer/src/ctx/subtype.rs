use crate::{
    ctx::Ctx,
    ctx::Log,
    error::TypeError,
    occurs_in::occurs_in,
    substitute::substitute,
    ty::{effect_expr::EffectExpr, Type},
};

impl Ctx {
    pub fn subtype(&self, sub: &Type, ty: &Type) -> Result<Ctx, TypeError> {
        let subtype_if = |pred: bool| {
            if pred {
                Ok(self.clone())
            } else {
                Err(TypeError::NotSubtype {
                    sub: sub.clone(),
                    ty: ty.clone(),
                })
            }
        };
        let ctx = match (sub, ty) {
            (Type::Variable(id), Type::Variable(id2)) if id == id2 => self.clone(),
            (Type::Number, Type::Number) => self.clone(),
            (Type::String, Type::String) => self.clone(),
            (Type::Existential(id), Type::Existential(id2)) if id == id2 => self.clone(),
            (Type::Existential(id), ty) => {
                if occurs_in(id, ty) {
                    return Err(TypeError::CircularExistential {
                        id: *id,
                        ty: ty.clone(),
                    });
                } else {
                    self.instantiate_subtype(id, ty)?
                }
            }
            (sub, Type::Existential(id)) => {
                if occurs_in(id, sub) {
                    return Err(TypeError::CircularExistential {
                        id: *id,
                        ty: ty.clone(),
                    });
                } else {
                    self.instantiate_supertype(sub, id)?
                }
            }

            // handling things must be under the instantiations of existential.
            (Type::Product(sub_types), Type::Product(types)) => {
                if sub_types
                    .iter()
                    .all(|sub_ty| types.iter().any(|ty| self.subtype(sub_ty, ty).is_ok()))
                {
                    self.clone()
                } else {
                    return Err(TypeError::NotSubtype {
                        sub: sub.clone(),
                        ty: ty.clone(),
                    });
                }
            }
            // TODO: return multi pass for error recovery?
            (Type::Product(sub_types), ty) => sub_types
                .iter()
                .find_map(|sub_ty| match self.subtype(sub_ty, ty) {
                    Ok(ctx) => Some(ctx),
                    Err(_) => None,
                })
                .ok_or(TypeError::NotSubtype {
                    sub: sub.clone(),
                    ty: ty.clone(),
                })?,
            (Type::Sum(sub_types), Type::Sum(types)) => {
                if types.iter().all(|ty| {
                    sub_types
                        .iter()
                        .any(|sub_ty| self.subtype(sub_ty, ty).is_ok())
                }) {
                    self.clone()
                } else {
                    return Err(TypeError::NotSubtype {
                        sub: sub.clone(),
                        ty: ty.clone(),
                    });
                }
            }
            // TODO: return multi pass for error recovery?
            (sub, Type::Sum(types)) => types
                .iter()
                .find_map(|ty| match self.subtype(sub, ty) {
                    Ok(ctx) => Some(ctx),
                    Err(_) => None,
                })
                .ok_or(TypeError::NotSubtype {
                    sub: sub.clone(),
                    ty: ty.clone(),
                })?,
            (
                Type::Function {
                    parameter: sub_parameter,
                    body: sub_body,
                },
                Type::Function { parameter, body },
            ) => {
                let theta = self.subtype(sub_parameter, parameter)?;
                theta.subtype(
                    &theta.substitute_from_ctx(body),
                    &theta.substitute_from_ctx(sub_body),
                )?
            }
            (Type::Vector(sub), Type::Vector(ty)) => self.subtype(sub, ty)?,
            (Type::Set(sub), Type::Set(ty)) => self.subtype(sub, ty)?,
            (Type::Variable(id), Type::Variable(id2)) => subtype_if(id == id2)?,
            (Type::ForAll { variable, body }, ty) => {
                let a = self.fresh_existential();
                self.add(Log::Marker(a))
                    .add(Log::Existential(a))
                    .subtype(&substitute(body, variable, &Type::Existential(a)), ty)?
                    .truncate_from(&Log::Marker(a))
                    .recover_effects()
            }
            (sub, Type::ForAll { variable, body }) => self
                .add(Log::Variable(*variable))
                .subtype(sub, body)?
                .truncate_from(&Log::Variable(*variable))
                .recover_effects(),

            (sub, Type::Label { item, label: _ }) => self.subtype(sub, item)?,
            (Type::Label { item, label: _ }, sup) => self.subtype(item, sup)?,
            (Type::Brand { item, brand: _ }, sup) => self.subtype(item, sup)?,
            // one without brand is not subtype of other with brand
            (Type::Infer(id), sup) => {
                self.store_inferred_type(id.clone(), sup.clone());
                self.clone()
            }
            (sub, Type::Infer(id)) => {
                self.store_inferred_type(id.clone(), sub.clone());
                self.clone()
            }

            (
                Type::Effectful { ty, effects },
                Type::Effectful {
                    ty: ty2,
                    effects: super_effects,
                },
            ) => {
                let theta = self.subtype(ty, ty2)?;
                theta.add_effects(&EffectExpr::Sub {
                    minuend: Box::new(effects.clone()),
                    subtrahend: Box::new(super_effects.clone()),
                })
            }
            (Type::Effectful { ty, effects }, ty2) => {
                let theta = self.subtype(ty, ty2)?;
                theta.add_effects(effects)
            }
            (sub, Type::Effectful { ty, effects: _ }) => self.subtype(sub, ty)?,
            (_, _) => {
                return Err(TypeError::NotSubtype {
                    sub: sub.clone(),
                    ty: ty.clone(),
                })
            }
        };
        Ok(ctx)
    }
}
