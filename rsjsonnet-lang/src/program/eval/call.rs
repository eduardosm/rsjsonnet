use super::super::{
    ir, BuiltInFunc, FuncData, FuncKind, FuncParams, ThunkData, ThunkEnv, ThunkEnvData,
};
use super::{EvalErrorKind, EvalResult, Evaluator, State, TraceItem};
use crate::gc::{Gc, GcView};
use crate::interner::InternedStr;
use crate::span::SpanId;

impl<'p> Evaluator<'_, 'p> {
    #[inline]
    pub(super) fn get_func_info(
        &self,
        func: &FuncData<'p>,
    ) -> (Option<InternedStr<'p>>, Option<Gc<ThunkEnv<'p>>>) {
        match func.kind {
            FuncKind::Identity { name } => (name, None),
            FuncKind::Normal { name, ref env, .. } => (name, Some(env.clone())),
            FuncKind::BuiltIn { name, .. } => (Some(name), None),
            FuncKind::Native { .. } => (None, None),
        }
    }

    pub(super) fn check_call_expr_args(
        &self,
        params: &FuncParams<'p>,
        positional_args: &[&'p ir::Expr<'p>],
        named_args: &[(InternedStr<'p>, SpanId, &'p ir::Expr<'p>)],
        call_env: GcView<ThunkEnv<'p>>,
        func_env: Option<Gc<ThunkEnv<'p>>>,
        call_span: SpanId,
    ) -> EvalResult<Box<[Gc<ThunkData<'p>>]>> {
        self.check_call_args_generic(
            params,
            positional_args,
            |this, expr| {
                this.program
                    .new_pending_expr_thunk(expr, Gc::from(&call_env), None)
            },
            named_args,
            |&(name, _, _)| name,
            |(_, name_span, _)| Some(*name_span),
            |this, (_, _, expr)| {
                this.program
                    .new_pending_expr_thunk(expr, Gc::from(&call_env), None)
            },
            func_env,
            Some(call_span),
        )
    }

    fn check_call_thunk_args(
        &self,
        params: &FuncParams<'p>,
        positional_args: &[GcView<ThunkData<'p>>],
        named_args: &[(InternedStr<'p>, GcView<ThunkData<'p>>)],
        func_env: Option<Gc<ThunkEnv<'p>>>,
    ) -> EvalResult<Box<[Gc<ThunkData<'p>>]>> {
        self.check_call_args_generic(
            params,
            positional_args,
            |_, thunk| Gc::from(thunk),
            named_args,
            |&(name, _)| name,
            |(_, _)| None,
            |_, (_, thunk)| Gc::from(thunk),
            func_env,
            None,
        )
    }

    #[inline]
    fn check_call_args_generic<PosArg, NamedArg>(
        &self,
        params: &FuncParams<'p>,
        positional_args: &[PosArg],
        pos_arg_thunk: impl Fn(&Self, &PosArg) -> Gc<ThunkData<'p>>,
        named_args: &[NamedArg],
        named_arg_name: impl Fn(&NamedArg) -> InternedStr<'p>,
        named_arg_name_span: impl Fn(&NamedArg) -> Option<SpanId>,
        named_arg_thunk: impl Fn(&Self, &NamedArg) -> Gc<ThunkData<'p>>,
        func_env: Option<Gc<ThunkEnv<'p>>>,
        call_span: Option<SpanId>,
    ) -> EvalResult<Box<[Gc<ThunkData<'p>>]>> {
        if positional_args.len() > params.order.len() {
            return Err(self.report_error(EvalErrorKind::TooManyCallArgs {
                span: call_span,
                num_params: params.order.len(),
            }));
        }

        // Handle positional arguments
        let mut args_thunks = Vec::with_capacity(params.order.len());
        args_thunks.extend(positional_args.iter().map(|arg| pos_arg_thunk(self, arg)));

        if args_thunks.len() == params.order.len() && named_args.is_empty() {
            // Fast path when all arguments are positional
            return Ok(args_thunks.into_boxed_slice());
        }

        // Handle named arguments into a temporary vector
        let mut named_args_tmp = vec![None; params.order.len() - args_thunks.len()];

        for named_arg in named_args.iter() {
            let param_name = named_arg_name(named_arg);
            let name_span = named_arg_name_span(named_arg);
            let Some(&param_i) = params.by_name.get(&param_name) else {
                return Err(self.report_error(EvalErrorKind::UnknownCallParam {
                    span: name_span,
                    param_name: param_name.value().into(),
                }));
            };
            if param_i < args_thunks.len() {
                return Err(self.report_error(EvalErrorKind::RepeatedCallParam {
                    span: name_span,
                    param_name: param_name.value().into(),
                }));
            }
            let arg_tmp = &mut named_args_tmp[param_i - args_thunks.len()];
            if arg_tmp.is_some() {
                return Err(self.report_error(EvalErrorKind::RepeatedCallParam {
                    span: name_span,
                    param_name: param_name.value().into(),
                }));
            }
            *arg_tmp = Some(named_arg_thunk(self, named_arg));
        }

        // Move named arguments from the temporary vector to the final vector
        let mut named_args_tmp = named_args_tmp.into_iter();
        while named_args_tmp.len() != 0 {
            if named_args_tmp.as_slice()[0].is_none() {
                // Let the next loop try to find default arguments.
                break;
            }
            let arg = named_args_tmp.next().unwrap().unwrap();
            args_thunks.push(arg);
        }

        if named_args_tmp.len() == 0 {
            // Fast path when all parameters are bound
            // without needing defaults.
            assert_eq!(args_thunks.len(), params.order.len());
            return Ok(args_thunks.into_boxed_slice());
        }

        // An environment is required to evaluate default arguments.
        let args_env = self.program.gc_alloc_view(ThunkEnv::new());

        for arg_tmp in named_args_tmp {
            if let Some(arg) = arg_tmp {
                args_thunks.push(arg);
            } else {
                let (param_name, default_arg) = params.order[args_thunks.len()];
                let Some(default_arg) = default_arg else {
                    return Err(self.report_error(EvalErrorKind::CallParamNotBound {
                        span: call_span,
                        param_name: param_name.value().into(),
                    }));
                };
                args_thunks.push(self.program.new_pending_expr_thunk(
                    default_arg,
                    Gc::from(&args_env),
                    None,
                ));
            }
        }

        let mut args_env_data = ThunkEnvData::new(func_env);
        for (&(arg_name, _), arg_thunk) in params.order.iter().zip(args_thunks.iter()) {
            args_env_data.set_var(arg_name, arg_thunk.clone());
        }

        args_env.set_data(args_env_data);

        assert_eq!(args_thunks.len(), params.order.len());
        Ok(args_thunks.into_boxed_slice())
    }

    #[inline]
    pub(super) fn check_thunk_args_and_execute_call(
        &mut self,
        func: &FuncData<'p>,
        positional_args: &[GcView<ThunkData<'p>>],
        named_args: &[(InternedStr<'p>, GcView<ThunkData<'p>>)],
        call_span: Option<SpanId>,
    ) -> EvalResult<()> {
        let (func_name, func_env) = self.get_func_info(func);
        let args_thunks =
            self.check_call_thunk_args(&func.params, positional_args, named_args, func_env)?;
        self.push_trace_item(TraceItem::Call {
            span: call_span,
            name: func_name,
        });
        self.execute_call(func, args_thunks);
        Ok(())
    }

    pub(super) fn execute_call(&mut self, func: &FuncData<'p>, args: Box<[Gc<ThunkData<'p>>]>) {
        match func.kind {
            FuncKind::Identity { .. } => {
                let [arg] = &*args else {
                    unreachable!();
                };
                self.state_stack.push(State::DoThunk(arg.view()));
            }
            FuncKind::Normal { body, ref env, .. } => {
                self.execute_normal_call(&func.params, body, env.clone(), args);
            }
            FuncKind::BuiltIn { kind, .. } => {
                self.execute_built_in_call(kind, &args);
            }
            FuncKind::Native { name, .. } => {
                self.execute_native_call(name, &args);
            }
        }
    }

    pub(super) fn execute_normal_call(
        &mut self,
        params: &FuncParams<'p>,
        body: &'p ir::Expr<'p>,
        env: Gc<ThunkEnv<'p>>,
        args: Box<[Gc<ThunkData<'p>>]>,
    ) {
        let inner_env = self.program.gc_alloc_view(ThunkEnv::new());
        let mut inner_env_data = ThunkEnvData::new(Some(env));
        for (&(arg_name, _), arg_thunk) in params.order.iter().zip(Vec::from(args)) {
            inner_env_data.set_var(arg_name, arg_thunk);
        }
        inner_env.set_data(inner_env_data);

        self.state_stack.push(State::Expr {
            expr: body,
            env: inner_env,
        });
    }

    fn execute_built_in_call(&mut self, kind: BuiltInFunc, args: &[Gc<ThunkData<'p>>]) {
        #[inline]
        fn check_num_args<'a, 'p, const N: usize>(
            args: &'a [Gc<ThunkData<'p>>],
        ) -> &'a [Gc<ThunkData<'p>>; N] {
            args.try_into().unwrap()
        }
        match kind {
            BuiltInFunc::ExtVar => {
                let [arg] = check_num_args(args);
                self.state_stack
                    .push(State::FnFallible(Self::do_std_ext_var));
                self.state_stack.push(State::DoThunk(arg.view()));
            }
            BuiltInFunc::Type => {
                let [arg] = check_num_args(args);
                self.state_stack
                    .push(State::FnInfallible(Self::do_std_type));
                self.state_stack.push(State::DoThunk(arg.view()));
            }
            BuiltInFunc::IsArray => {
                let [arg] = check_num_args(args);
                self.state_stack
                    .push(State::FnInfallible(Self::do_std_is_array));
                self.state_stack.push(State::DoThunk(arg.view()));
            }
            BuiltInFunc::IsBoolean => {
                let [arg] = check_num_args(args);
                self.state_stack
                    .push(State::FnInfallible(Self::do_std_is_boolean));
                self.state_stack.push(State::DoThunk(arg.view()));
            }
            BuiltInFunc::IsFunction => {
                let [arg] = check_num_args(args);
                self.state_stack
                    .push(State::FnInfallible(Self::do_std_is_function));
                self.state_stack.push(State::DoThunk(arg.view()));
            }
            BuiltInFunc::IsNumber => {
                let [arg] = check_num_args(args);
                self.state_stack
                    .push(State::FnInfallible(Self::do_std_is_number));
                self.state_stack.push(State::DoThunk(arg.view()));
            }
            BuiltInFunc::IsObject => {
                let [arg] = check_num_args(args);
                self.state_stack
                    .push(State::FnInfallible(Self::do_std_is_object));
                self.state_stack.push(State::DoThunk(arg.view()));
            }
            BuiltInFunc::IsString => {
                let [arg] = check_num_args(args);
                self.state_stack
                    .push(State::FnInfallible(Self::do_std_is_string));
                self.state_stack.push(State::DoThunk(arg.view()));
            }
            BuiltInFunc::Length => {
                let [arg] = check_num_args(args);
                self.state_stack
                    .push(State::FnFallible(Self::do_std_length));
                self.state_stack.push(State::DoThunk(arg.view()));
            }
            BuiltInFunc::Prune => {
                let [arg] = check_num_args(args);
                self.state_stack.push(State::StdPruneValue);
                self.state_stack.push(State::DoThunk(arg.view()));
            }
            BuiltInFunc::ObjectHasEx => {
                let [arg0, arg1, arg2] = check_num_args(args);
                self.state_stack
                    .push(State::FnFallible(Self::do_std_object_has_ex));
                self.state_stack.push(State::DoThunk(arg2.view()));
                self.state_stack.push(State::DoThunk(arg1.view()));
                self.state_stack.push(State::DoThunk(arg0.view()));
            }
            BuiltInFunc::ObjectFieldsEx => {
                let [arg0, arg1] = check_num_args(args);
                self.state_stack
                    .push(State::FnFallible(Self::do_std_object_fields_ex));
                self.state_stack.push(State::DoThunk(arg1.view()));
                self.state_stack.push(State::DoThunk(arg0.view()));
            }
            BuiltInFunc::MapWithKey => {
                let [arg0, arg1] = check_num_args(args);
                self.state_stack
                    .push(State::FnFallible(Self::do_std_map_with_key));
                self.state_stack.push(State::DoThunk(arg1.view()));
                self.state_stack.push(State::DoThunk(arg0.view()));
            }
            BuiltInFunc::PrimitiveEquals => {
                let [arg0, arg1] = check_num_args(args);
                self.state_stack
                    .push(State::FnFallible(Self::do_std_primitive_equals));
                self.state_stack.push(State::DoThunk(arg1.view()));
                self.state_stack.push(State::DoThunk(arg0.view()));
            }
            BuiltInFunc::Equals => {
                let [arg0, arg1] = check_num_args(args);
                self.state_stack.push(State::BoolToValue);
                self.state_stack.push(State::EqualsValue);
                self.state_stack.push(State::DoThunk(arg1.view()));
                self.state_stack.push(State::DoThunk(arg0.view()));
            }
            BuiltInFunc::Compare => {
                let [arg0, arg1] = check_num_args(args);
                self.state_stack.push(State::CmpOrdToIntValueThreeWay);
                self.state_stack.push(State::CompareValue);
                self.state_stack.push(State::DoThunk(arg1.view()));
                self.state_stack.push(State::DoThunk(arg0.view()));
            }
            BuiltInFunc::CompareArray => {
                let [arg0, arg1] = check_num_args(args);
                self.state_stack
                    .push(State::FnFallible(Self::do_std_compare_array));
                self.state_stack.push(State::DoThunk(arg1.view()));
                self.state_stack.push(State::DoThunk(arg0.view()));
            }
            BuiltInFunc::Exponent => {
                let [arg] = check_num_args(args);
                self.state_stack
                    .push(State::FnFallible(Self::do_std_exponent));
                self.state_stack.push(State::DoThunk(arg.view()));
            }
            BuiltInFunc::Mantissa => {
                let [arg] = check_num_args(args);
                self.state_stack
                    .push(State::FnFallible(Self::do_std_mantissa));
                self.state_stack.push(State::DoThunk(arg.view()));
            }
            BuiltInFunc::Floor => {
                let [arg] = check_num_args(args);
                self.state_stack.push(State::FnFallible(Self::do_std_floor));
                self.state_stack.push(State::DoThunk(arg.view()));
            }
            BuiltInFunc::Ceil => {
                let [arg] = check_num_args(args);
                self.state_stack.push(State::FnFallible(Self::do_std_ceil));
                self.state_stack.push(State::DoThunk(arg.view()));
            }
            BuiltInFunc::Modulo => {
                let [arg0, arg1] = check_num_args(args);
                self.state_stack
                    .push(State::FnFallible(Self::do_std_modulo));
                self.state_stack.push(State::DoThunk(arg1.view()));
                self.state_stack.push(State::DoThunk(arg0.view()));
            }
            BuiltInFunc::Pow => {
                let [arg0, arg1] = check_num_args(args);
                self.state_stack.push(State::FnFallible(Self::do_std_pow));
                self.state_stack.push(State::DoThunk(arg1.view()));
                self.state_stack.push(State::DoThunk(arg0.view()));
            }
            BuiltInFunc::Exp => {
                let [arg] = check_num_args(args);
                self.state_stack.push(State::FnFallible(Self::do_std_exp));
                self.state_stack.push(State::DoThunk(arg.view()));
            }
            BuiltInFunc::Log => {
                let [arg] = check_num_args(args);
                self.state_stack.push(State::FnFallible(Self::do_std_log));
                self.state_stack.push(State::DoThunk(arg.view()));
            }
            BuiltInFunc::Log2 => {
                let [arg] = check_num_args(args);
                self.state_stack.push(State::FnFallible(Self::do_std_log2));
                self.state_stack.push(State::DoThunk(arg.view()));
            }
            BuiltInFunc::Log10 => {
                let [arg] = check_num_args(args);
                self.state_stack.push(State::FnFallible(Self::do_std_log10));
                self.state_stack.push(State::DoThunk(arg.view()));
            }
            BuiltInFunc::Sqrt => {
                let [arg] = check_num_args(args);
                self.state_stack.push(State::FnFallible(Self::do_std_sqrt));
                self.state_stack.push(State::DoThunk(arg.view()));
            }
            BuiltInFunc::Sin => {
                let [arg] = check_num_args(args);
                self.state_stack.push(State::FnFallible(Self::do_std_sin));
                self.state_stack.push(State::DoThunk(arg.view()));
            }
            BuiltInFunc::Cos => {
                let [arg] = check_num_args(args);
                self.state_stack.push(State::FnFallible(Self::do_std_cos));
                self.state_stack.push(State::DoThunk(arg.view()));
            }
            BuiltInFunc::Tan => {
                let [arg] = check_num_args(args);
                self.state_stack.push(State::FnFallible(Self::do_std_tan));
                self.state_stack.push(State::DoThunk(arg.view()));
            }
            BuiltInFunc::Asin => {
                let [arg] = check_num_args(args);
                self.state_stack.push(State::FnFallible(Self::do_std_asin));
                self.state_stack.push(State::DoThunk(arg.view()));
            }
            BuiltInFunc::Acos => {
                let [arg] = check_num_args(args);
                self.state_stack.push(State::FnFallible(Self::do_std_acos));
                self.state_stack.push(State::DoThunk(arg.view()));
            }
            BuiltInFunc::Atan => {
                let [arg] = check_num_args(args);
                self.state_stack.push(State::FnFallible(Self::do_std_atan));
                self.state_stack.push(State::DoThunk(arg.view()));
            }
            BuiltInFunc::Atan2 => {
                let [arg0, arg1] = check_num_args(args);
                self.state_stack.push(State::FnFallible(Self::do_std_atan2));
                self.state_stack.push(State::DoThunk(arg1.view()));
                self.state_stack.push(State::DoThunk(arg0.view()));
            }
            BuiltInFunc::Deg2Rad => {
                let [arg] = check_num_args(args);
                self.state_stack
                    .push(State::FnFallible(Self::do_std_deg2rad));
                self.state_stack.push(State::DoThunk(arg.view()));
            }
            BuiltInFunc::Rad2Deg => {
                let [arg] = check_num_args(args);
                self.state_stack
                    .push(State::FnFallible(Self::do_std_rad2deg));
                self.state_stack.push(State::DoThunk(arg.view()));
            }
            BuiltInFunc::Hypot => {
                let [arg0, arg1] = check_num_args(args);
                self.state_stack.push(State::FnFallible(Self::do_std_hypot));
                self.state_stack.push(State::DoThunk(arg1.view()));
                self.state_stack.push(State::DoThunk(arg0.view()));
            }
            BuiltInFunc::IsEven => {
                let [arg] = check_num_args(args);
                self.state_stack
                    .push(State::FnFallible(Self::do_std_is_even));
                self.state_stack.push(State::DoThunk(arg.view()));
            }
            BuiltInFunc::IsOdd => {
                let [arg] = check_num_args(args);
                self.state_stack
                    .push(State::FnFallible(Self::do_std_is_odd));
                self.state_stack.push(State::DoThunk(arg.view()));
            }
            BuiltInFunc::IsInteger => {
                let [arg] = check_num_args(args);
                self.state_stack
                    .push(State::FnFallible(Self::do_std_is_integer));
                self.state_stack.push(State::DoThunk(arg.view()));
            }
            BuiltInFunc::IsDecimal => {
                let [arg] = check_num_args(args);
                self.state_stack
                    .push(State::FnFallible(Self::do_std_is_decimal));
                self.state_stack.push(State::DoThunk(arg.view()));
            }
            BuiltInFunc::AssertEqual => {
                let [arg0, arg1] = check_num_args(args);
                self.state_stack
                    .push(State::FnInfallible(Self::do_std_assert_equal));
                self.state_stack.push(State::DoThunk(arg1.view()));
                self.state_stack.push(State::DoThunk(arg0.view()));
            }
            BuiltInFunc::ToString => {
                let [arg] = check_num_args(args);
                self.state_stack.push(State::CoerceToStringValue);
                self.state_stack.push(State::DoThunk(arg.view()));
            }
            BuiltInFunc::Codepoint => {
                let [arg] = check_num_args(args);
                self.state_stack
                    .push(State::FnFallible(Self::do_std_codepoint));
                self.state_stack.push(State::DoThunk(arg.view()));
            }
            BuiltInFunc::Char => {
                let [arg] = check_num_args(args);
                self.state_stack.push(State::FnFallible(Self::do_std_char));
                self.state_stack.push(State::DoThunk(arg.view()));
            }
            BuiltInFunc::Substr => {
                let [arg0, arg1, arg2] = check_num_args(args);
                self.state_stack
                    .push(State::FnFallible(Self::do_std_substr));
                self.state_stack.push(State::DoThunk(arg2.view()));
                self.state_stack.push(State::DoThunk(arg1.view()));
                self.state_stack.push(State::DoThunk(arg0.view()));
            }
            BuiltInFunc::FindSubstr => {
                let [arg0, arg1] = check_num_args(args);
                self.state_stack
                    .push(State::FnFallible(Self::do_std_find_substr));
                self.state_stack.push(State::DoThunk(arg1.view()));
                self.state_stack.push(State::DoThunk(arg0.view()));
            }
            BuiltInFunc::StartsWith => {
                let [arg0, arg1] = check_num_args(args);
                self.state_stack
                    .push(State::FnFallible(Self::do_std_starts_with));
                self.state_stack.push(State::DoThunk(arg1.view()));
                self.state_stack.push(State::DoThunk(arg0.view()));
            }
            BuiltInFunc::EndsWith => {
                let [arg0, arg1] = check_num_args(args);
                self.state_stack
                    .push(State::FnFallible(Self::do_std_ends_with));
                self.state_stack.push(State::DoThunk(arg1.view()));
                self.state_stack.push(State::DoThunk(arg0.view()));
            }
            BuiltInFunc::StripChars => {
                let [arg0, arg1] = check_num_args(args);
                self.state_stack
                    .push(State::FnFallible(Self::do_std_strip_chars));
                self.state_stack.push(State::DoThunk(arg1.view()));
                self.state_stack.push(State::DoThunk(arg0.view()));
            }
            BuiltInFunc::LStripChars => {
                let [arg0, arg1] = check_num_args(args);
                self.state_stack
                    .push(State::FnFallible(Self::do_std_lstrip_chars));
                self.state_stack.push(State::DoThunk(arg1.view()));
                self.state_stack.push(State::DoThunk(arg0.view()));
            }
            BuiltInFunc::RStripChars => {
                let [arg0, arg1] = check_num_args(args);
                self.state_stack
                    .push(State::FnFallible(Self::do_std_rstrip_chars));
                self.state_stack.push(State::DoThunk(arg1.view()));
                self.state_stack.push(State::DoThunk(arg0.view()));
            }
            BuiltInFunc::Split => {
                let [arg0, arg1] = check_num_args(args);
                self.state_stack.push(State::FnFallible(Self::do_std_split));
                self.state_stack.push(State::DoThunk(arg1.view()));
                self.state_stack.push(State::DoThunk(arg0.view()));
            }
            BuiltInFunc::SplitLimit => {
                let [arg0, arg1, arg2] = check_num_args(args);
                self.state_stack
                    .push(State::FnFallible(Self::do_std_split_limit));
                self.state_stack.push(State::DoThunk(arg2.view()));
                self.state_stack.push(State::DoThunk(arg1.view()));
                self.state_stack.push(State::DoThunk(arg0.view()));
            }
            BuiltInFunc::SplitLimitR => {
                let [arg0, arg1, arg2] = check_num_args(args);
                self.state_stack
                    .push(State::FnFallible(Self::do_std_split_limit_r));
                self.state_stack.push(State::DoThunk(arg2.view()));
                self.state_stack.push(State::DoThunk(arg1.view()));
                self.state_stack.push(State::DoThunk(arg0.view()));
            }
            BuiltInFunc::StrReplace => {
                let [arg0, arg1, arg2] = check_num_args(args);
                self.state_stack
                    .push(State::FnFallible(Self::do_std_str_replace));
                self.state_stack.push(State::DoThunk(arg2.view()));
                self.state_stack.push(State::DoThunk(arg1.view()));
                self.state_stack.push(State::DoThunk(arg0.view()));
            }
            BuiltInFunc::Trim => {
                let [arg] = check_num_args(args);
                self.state_stack.push(State::FnFallible(Self::do_std_trim));
                self.state_stack.push(State::DoThunk(arg.view()));
            }
            BuiltInFunc::EqualsIgnoreCase => {
                let [arg0, arg1] = check_num_args(args);
                self.state_stack
                    .push(State::FnFallible(Self::do_std_equals_ignore_case));
                self.state_stack.push(State::DoThunk(arg1.view()));
                self.state_stack.push(State::DoThunk(arg0.view()));
            }
            BuiltInFunc::AsciiUpper => {
                let [arg] = check_num_args(args);
                self.state_stack
                    .push(State::FnFallible(Self::do_std_ascii_upper));
                self.state_stack.push(State::DoThunk(arg.view()));
            }
            BuiltInFunc::AsciiLower => {
                let [arg] = check_num_args(args);
                self.state_stack
                    .push(State::FnFallible(Self::do_std_ascii_lower));
                self.state_stack.push(State::DoThunk(arg.view()));
            }
            BuiltInFunc::StringChars => {
                let [arg] = check_num_args(args);
                self.state_stack
                    .push(State::FnFallible(Self::do_std_string_chars));
                self.state_stack.push(State::DoThunk(arg.view()));
            }
            BuiltInFunc::Format => {
                let [arg0, arg1] = check_num_args(args);
                self.state_stack.push(State::StdFormat);
                self.state_stack.push(State::DoThunk(arg1.view()));
                self.state_stack.push(State::DoThunk(arg0.view()));
            }
            BuiltInFunc::EscapeStringJson => {
                let [arg] = check_num_args(args);
                self.state_stack
                    .push(State::FnInfallible(Self::do_std_escape_string_json));
                self.state_stack.push(State::CoerceToString);
                self.state_stack.push(State::DoThunk(arg.view()));
            }
            BuiltInFunc::EscapeStringPython => {
                let [arg] = check_num_args(args);
                self.state_stack
                    .push(State::FnInfallible(Self::do_std_escape_string_python));
                self.state_stack.push(State::CoerceToString);
                self.state_stack.push(State::DoThunk(arg.view()));
            }
            BuiltInFunc::EscapeStringBash => {
                let [arg] = check_num_args(args);
                self.state_stack
                    .push(State::FnInfallible(Self::do_std_escape_string_bash));
                self.state_stack.push(State::CoerceToString);
                self.state_stack.push(State::DoThunk(arg.view()));
            }
            BuiltInFunc::EscapeStringDollars => {
                let [arg] = check_num_args(args);
                self.state_stack
                    .push(State::FnInfallible(Self::do_std_escape_string_dollars));
                self.state_stack.push(State::CoerceToString);
                self.state_stack.push(State::DoThunk(arg.view()));
            }
            BuiltInFunc::EscapeStringXml => {
                let [arg] = check_num_args(args);
                self.state_stack
                    .push(State::FnInfallible(Self::do_std_escape_string_xml));
                self.state_stack.push(State::CoerceToString);
                self.state_stack.push(State::DoThunk(arg.view()));
            }
            BuiltInFunc::ParseInt => {
                let [arg] = check_num_args(args);
                self.state_stack
                    .push(State::FnFallible(Self::do_std_parse_int));
                self.state_stack.push(State::DoThunk(arg.view()));
            }
            BuiltInFunc::ParseOctal => {
                let [arg] = check_num_args(args);
                self.state_stack
                    .push(State::FnFallible(Self::do_std_parse_octal));
                self.state_stack.push(State::DoThunk(arg.view()));
            }
            BuiltInFunc::ParseHex => {
                let [arg] = check_num_args(args);
                self.state_stack
                    .push(State::FnFallible(Self::do_std_parse_hex));
                self.state_stack.push(State::DoThunk(arg.view()));
            }
            BuiltInFunc::ParseJson => {
                let [arg] = check_num_args(args);
                self.state_stack
                    .push(State::FnFallible(Self::do_std_parse_json));
                self.state_stack.push(State::DoThunk(arg.view()));
            }
            BuiltInFunc::ParseYaml => {
                let [arg] = check_num_args(args);
                self.state_stack
                    .push(State::FnFallible(Self::do_std_parse_yaml));
                self.state_stack.push(State::DoThunk(arg.view()));
            }
            BuiltInFunc::EncodeUtf8 => {
                self.state_stack
                    .push(State::FnFallible(Self::do_std_encode_utf8));
                self.state_stack.push(State::DoThunk(args[0].view()));
            }
            BuiltInFunc::DecodeUtf8 => {
                let [arg] = check_num_args(args);
                self.state_stack
                    .push(State::FnFallible(Self::do_std_decode_utf8));
                self.state_stack.push(State::DoThunk(arg.view()));
            }
            BuiltInFunc::ManifestIni => {
                let [arg] = check_num_args(args);
                self.state_stack.push(State::StdManifestIni);
                self.state_stack.push(State::DoThunk(arg.view()));
            }
            BuiltInFunc::ManifestPython => {
                let [arg] = check_num_args(args);
                self.state_stack.push(State::StdManifestPython);
                self.state_stack.push(State::DoThunk(arg.view()));
            }
            BuiltInFunc::ManifestPythonVars => {
                let [arg] = check_num_args(args);
                self.state_stack.push(State::StdManifestPythonVars);
                self.state_stack.push(State::DoThunk(arg.view()));
            }
            BuiltInFunc::ManifestJsonEx => {
                let [arg0, arg1, arg2, arg3] = check_num_args(args);
                self.state_stack.push(State::StdManifestJsonEx);
                self.state_stack.push(State::DoThunk(arg3.view()));
                self.state_stack.push(State::DoThunk(arg2.view()));
                self.state_stack.push(State::DoThunk(arg1.view()));
                self.state_stack.push(State::DoThunk(arg0.view()));
            }
            BuiltInFunc::ManifestYamlDoc => {
                let [arg0, arg1, arg2] = check_num_args(args);
                self.state_stack.push(State::StdManifestYamlDoc);
                self.state_stack.push(State::DoThunk(arg2.view()));
                self.state_stack.push(State::DoThunk(arg1.view()));
                self.state_stack.push(State::DoThunk(arg0.view()));
            }
            BuiltInFunc::ManifestYamlStream => {
                let [arg0, arg1, arg2, arg3] = check_num_args(args);
                self.state_stack.push(State::StdManifestYamlStream);
                self.state_stack.push(State::DoThunk(arg3.view()));
                self.state_stack.push(State::DoThunk(arg2.view()));
                self.state_stack.push(State::DoThunk(arg1.view()));
                self.state_stack.push(State::DoThunk(arg0.view()));
            }
            BuiltInFunc::ManifestXmlJsonml => {
                let [arg] = check_num_args(args);
                self.state_stack.push(State::StdManifestXmlJsonml);
                self.state_stack.push(State::DoThunk(arg.view()));
            }
            BuiltInFunc::ManifestTomlEx => {
                let [arg0, arg1] = check_num_args(args);
                self.state_stack.push(State::StdManifestTomlEx);
                self.state_stack.push(State::DoThunk(arg1.view()));
                self.state_stack.push(State::DoThunk(arg0.view()));
            }
            BuiltInFunc::MakeArray => {
                let [arg0, arg1] = check_num_args(args);
                self.state_stack
                    .push(State::FnFallible(Self::do_std_make_array));
                self.state_stack.push(State::DoThunk(arg1.view()));
                self.state_stack.push(State::DoThunk(arg0.view()));
            }
            BuiltInFunc::Member => {
                let [arg0, arg1] = check_num_args(args);
                self.state_stack
                    .push(State::StdMember { value: arg1.view() });
                self.state_stack.push(State::DoThunk(arg0.view()));
            }
            BuiltInFunc::Count => {
                let [arg0, arg1] = check_num_args(args);
                self.state_stack
                    .push(State::StdCount { value: arg1.view() });
                self.state_stack.push(State::DoThunk(arg0.view()));
            }
            BuiltInFunc::Find => {
                let [arg0, arg1] = check_num_args(args);
                self.state_stack.push(State::StdFind { value: arg0.view() });
                self.state_stack.push(State::DoThunk(arg1.view()));
            }
            BuiltInFunc::Map => {
                let [arg0, arg1] = check_num_args(args);
                self.state_stack.push(State::FnFallible(Self::do_std_map));
                self.state_stack.push(State::DoThunk(arg1.view()));
                self.state_stack.push(State::DoThunk(arg0.view()));
            }
            BuiltInFunc::MapWithIndex => {
                let [arg0, arg1] = check_num_args(args);
                self.state_stack
                    .push(State::FnFallible(Self::do_std_map_with_index));
                self.state_stack.push(State::DoThunk(arg1.view()));
                self.state_stack.push(State::DoThunk(arg0.view()));
            }
            BuiltInFunc::FilterMap => {
                let [arg0, arg1, arg2] = check_num_args(args);
                self.state_stack.push(State::StdFilterMap);
                self.state_stack.push(State::DoThunk(arg2.view()));
                self.state_stack.push(State::DoThunk(arg1.view()));
                self.state_stack.push(State::DoThunk(arg0.view()));
            }
            BuiltInFunc::FlatMap => {
                let [arg0, arg1] = check_num_args(args);
                self.state_stack
                    .push(State::FnFallible(Self::do_std_flat_map));
                self.state_stack.push(State::DoThunk(arg1.view()));
                self.state_stack.push(State::DoThunk(arg0.view()));
            }
            BuiltInFunc::Filter => {
                let [arg0, arg1] = check_num_args(args);
                self.state_stack.push(State::StdFilter);
                self.state_stack.push(State::DoThunk(arg1.view()));
                self.state_stack.push(State::DoThunk(arg0.view()));
            }
            BuiltInFunc::Foldl => {
                let [arg0, arg1, arg2] = check_num_args(args);
                self.state_stack.push(State::StdFoldl { init: arg2.view() });
                self.state_stack.push(State::DoThunk(arg1.view()));
                self.state_stack.push(State::DoThunk(arg0.view()));
            }
            BuiltInFunc::Foldr => {
                let [arg0, arg1, arg2] = check_num_args(args);
                self.state_stack.push(State::StdFoldr { init: arg2.view() });
                self.state_stack.push(State::DoThunk(arg1.view()));
                self.state_stack.push(State::DoThunk(arg0.view()));
            }
            BuiltInFunc::Range => {
                let [arg0, arg1] = check_num_args(args);
                self.state_stack.push(State::FnFallible(Self::do_std_range));
                self.state_stack.push(State::DoThunk(arg1.view()));
                self.state_stack.push(State::DoThunk(arg0.view()));
            }
            BuiltInFunc::Repeat => {
                let [arg0, arg1] = check_num_args(args);
                self.state_stack
                    .push(State::FnFallible(Self::do_std_repeat));
                self.state_stack.push(State::DoThunk(arg1.view()));
                self.state_stack.push(State::DoThunk(arg0.view()));
            }
            BuiltInFunc::Slice => {
                let [arg0, arg1, arg2, arg3] = check_num_args(args);
                self.state_stack.push(State::FnFallible(Self::do_std_slice));
                self.state_stack.push(State::DoThunk(arg3.view()));
                self.state_stack.push(State::DoThunk(arg2.view()));
                self.state_stack.push(State::DoThunk(arg1.view()));
                self.state_stack.push(State::DoThunk(arg0.view()));
            }
            BuiltInFunc::Join => {
                let [arg0, arg1] = check_num_args(args);
                self.state_stack.push(State::StdJoin);
                self.state_stack.push(State::DoThunk(arg1.view()));
                self.state_stack.push(State::DoThunk(arg0.view()));
            }
            BuiltInFunc::DeepJoin => {
                let [arg] = check_num_args(args);
                self.state_stack
                    .push(State::FnFallible(Self::do_std_deep_join));
                self.state_stack.push(State::DoThunk(arg.view()));
            }
            BuiltInFunc::FlattenArrays => {
                let [arg] = check_num_args(args);
                self.state_stack
                    .push(State::FnFallible(Self::do_std_flatten_arrays));
                self.state_stack.push(State::DoThunk(arg.view()));
            }
            BuiltInFunc::Reverse => {
                let [arg] = check_num_args(args);
                self.state_stack
                    .push(State::FnFallible(Self::do_std_reverse));
                self.state_stack.push(State::DoThunk(arg.view()));
            }
            BuiltInFunc::Sort => {
                let [arg0, arg1] = check_num_args(args);
                self.state_stack.push(State::StdSort);
                self.state_stack.push(State::DoThunk(arg1.view()));
                self.state_stack.push(State::DoThunk(arg0.view()));
            }
            BuiltInFunc::Uniq => {
                let [arg0, arg1] = check_num_args(args);
                self.state_stack.push(State::StdUniq);
                self.state_stack.push(State::DoThunk(arg1.view()));
                self.state_stack.push(State::DoThunk(arg0.view()));
            }
            BuiltInFunc::All => {
                let [arg] = check_num_args(args);
                self.state_stack.push(State::StdAll);
                self.state_stack.push(State::DoThunk(arg.view()));
            }
            BuiltInFunc::Any => {
                let [arg] = check_num_args(args);
                self.state_stack.push(State::StdAny);
                self.state_stack.push(State::DoThunk(arg.view()));
            }
            BuiltInFunc::Sum => {
                let [arg] = check_num_args(args);
                self.state_stack.push(State::StdSum);
                self.state_stack.push(State::DoThunk(arg.view()));
            }
            BuiltInFunc::Avg => {
                let [arg] = check_num_args(args);
                self.state_stack.push(State::StdAvg);
                self.state_stack.push(State::DoThunk(arg.view()));
            }
            BuiltInFunc::MinArray => {
                let [arg0, arg1, arg2] = check_num_args(args);
                self.state_stack.push(State::StdMinArray {
                    on_empty: arg2.view(),
                });
                self.state_stack.push(State::DoThunk(arg1.view()));
                self.state_stack.push(State::DoThunk(arg0.view()));
            }
            BuiltInFunc::MaxArray => {
                let [arg0, arg1, arg2] = check_num_args(args);
                self.state_stack.push(State::StdMaxArray {
                    on_empty: arg2.view(),
                });
                self.state_stack.push(State::DoThunk(arg1.view()));
                self.state_stack.push(State::DoThunk(arg0.view()));
            }
            BuiltInFunc::Contains => {
                let [arg0, arg1] = check_num_args(args);
                self.state_stack
                    .push(State::StdContains { value: arg1.view() });
                self.state_stack.push(State::DoThunk(arg0.view()));
            }
            BuiltInFunc::Set => {
                let [arg0, arg1] = check_num_args(args);
                self.state_stack.push(State::StdSet);
                self.state_stack.push(State::DoThunk(arg1.view()));
                self.state_stack.push(State::DoThunk(arg0.view()));
            }
            BuiltInFunc::SetInter => {
                let [arg0, arg1, arg2] = check_num_args(args);
                self.state_stack.push(State::StdSetInter);
                self.state_stack.push(State::DoThunk(arg2.view()));
                self.state_stack.push(State::DoThunk(arg1.view()));
                self.state_stack.push(State::DoThunk(arg0.view()));
            }
            BuiltInFunc::SetUnion => {
                let [arg0, arg1, arg2] = check_num_args(args);
                self.state_stack.push(State::StdSetUnion);
                self.state_stack.push(State::DoThunk(arg2.view()));
                self.state_stack.push(State::DoThunk(arg1.view()));
                self.state_stack.push(State::DoThunk(arg0.view()));
            }
            BuiltInFunc::SetDiff => {
                let [arg0, arg1, arg2] = check_num_args(args);
                self.state_stack.push(State::StdSetDiff);
                self.state_stack.push(State::DoThunk(arg2.view()));
                self.state_stack.push(State::DoThunk(arg1.view()));
                self.state_stack.push(State::DoThunk(arg0.view()));
            }
            BuiltInFunc::SetMember => {
                let [arg0, arg1, arg2] = check_num_args(args);
                self.state_stack
                    .push(State::StdSetMember { x: arg0.view() });
                self.state_stack.push(State::DoThunk(arg2.view()));
                self.state_stack.push(State::DoThunk(arg1.view()));
            }
            BuiltInFunc::Base64 => {
                let [arg] = check_num_args(args);
                self.state_stack.push(State::StdBase64);
                self.state_stack.push(State::DoThunk(arg.view()));
            }
            BuiltInFunc::Base64DecodeBytes => {
                let [arg] = check_num_args(args);
                self.state_stack.push(State::StdBase64DecodeBytes);
                self.state_stack.push(State::DoThunk(arg.view()));
            }
            BuiltInFunc::Base64Decode => {
                let [arg] = check_num_args(args);
                self.state_stack.push(State::StdBase64Decode);
                self.state_stack.push(State::DoThunk(arg.view()));
            }
            BuiltInFunc::Md5 => {
                let [arg] = check_num_args(args);
                self.state_stack.push(State::FnFallible(Self::do_std_md5));
                self.state_stack.push(State::DoThunk(arg.view()));
            }
            BuiltInFunc::Sha1 => {
                let [arg] = check_num_args(args);
                self.state_stack.push(State::FnFallible(Self::do_std_sha1));
                self.state_stack.push(State::DoThunk(arg.view()));
            }
            BuiltInFunc::Sha256 => {
                let [arg] = check_num_args(args);
                self.state_stack
                    .push(State::FnFallible(Self::do_std_sha256));
                self.state_stack.push(State::DoThunk(arg.view()));
            }
            BuiltInFunc::Sha512 => {
                let [arg] = check_num_args(args);
                self.state_stack
                    .push(State::FnFallible(Self::do_std_sha512));
                self.state_stack.push(State::DoThunk(arg.view()));
            }
            BuiltInFunc::Sha3 => {
                let [arg] = check_num_args(args);
                self.state_stack.push(State::FnFallible(Self::do_std_sha3));
                self.state_stack.push(State::DoThunk(arg.view()));
            }
            BuiltInFunc::MergePatch => {
                let [arg0, arg1] = check_num_args(args);
                self.state_stack.push(State::StdMergePatchValue);
                self.state_stack.push(State::DoThunk(arg1.view()));
                self.state_stack.push(State::DoThunk(arg0.view()));
            }
            BuiltInFunc::Mod => {
                let [arg0, arg1] = check_num_args(args);
                self.state_stack.push(State::FnFallible(Self::do_std_mod));
                self.state_stack.push(State::DoThunk(arg1.view()));
                self.state_stack.push(State::DoThunk(arg0.view()));
            }
            BuiltInFunc::Native => {
                let [arg] = check_num_args(args);
                self.state_stack
                    .push(State::FnFallible(Self::do_std_native));
                self.state_stack.push(State::DoThunk(arg.view()));
            }
            BuiltInFunc::Trace => {
                let [arg0, arg1] = check_num_args(args);
                self.state_stack.push(State::FnFallible(Self::do_std_trace));
                self.state_stack.push(State::DoThunk(arg0.view()));
                self.state_stack.push(State::DoThunk(arg1.view()));
            }
        }
    }

    fn execute_native_call(&mut self, name: InternedStr<'p>, args: &[Gc<ThunkData<'p>>]) {
        self.state_stack.push(State::ExecNativeCall {
            name,
            args: args.iter().map(Gc::view).collect(),
        });

        for arg in args.iter().rev() {
            self.state_stack.push(State::DiscardValue);
            self.state_stack.push(State::DeepValue);
            self.state_stack.push(State::DoThunk(arg.view()));
        }
    }
}
