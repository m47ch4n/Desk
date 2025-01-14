use std::{ops::DerefMut, sync::Arc, time::Duration};

use types::{Effect, Type};

use crate::{
    dprocess_info::DProcessInfo,
    effect_handler::{EffectHandler, HaltProcess, SendMessage},
    interpreter::Interpreter,
    interpreter_output::InterpreterOutput,
    status::DProcessStatus,
    timer::Timer,
    value::Value,
    vm_ref::VmRef,
};

use super::DProcess;

impl DProcess {
    /// Execute the interpreter.
    pub fn reduce(&self, vm: VmRef, target_duration: &Duration) -> ProcessOutput {
        // lock both to prevent invalid state.
        let (mut interpreter, mut status) = self.lock_interpreter_and_status();
        match interpreter.reduce(target_duration) {
            Ok(output) => match output {
                InterpreterOutput::Returned(value) => {
                    let value = Arc::new(value);
                    // Don't update status like `*status = new_status`.
                    self.update_status(vm, &mut status, DProcessStatus::Returned(value.clone()));
                    ProcessOutput::Returned(value)
                }
                InterpreterOutput::Performed { input, effect } => {
                    self.handle_effect(vm, interpreter, status, effect, input)
                }
                InterpreterOutput::Running => ProcessOutput::Running,
            },
            Err(err) => {
                let err = Arc::new(err);
                self.update_status(vm, &mut status, DProcessStatus::Crashed(err.clone()));
                ProcessOutput::Crashed(err)
            }
        }
    }

    pub fn handle_effect(
        &self,
        vm: VmRef,
        mut interpreter: impl DerefMut<Target = Box<dyn Interpreter>>,
        mut status: impl DerefMut<Target = DProcessStatus>,
        effect: Effect,
        input: Value,
    ) -> ProcessOutput {
        // unwrap is safe because Desk plugins must ensure to .
        // clone is cheap.
        let handler = self.read_effect_handlers().0.get(&effect).unwrap().clone();
        match handler {
            EffectHandler::Immediate(handler) => {
                let output = handler.to_output(&input);
                interpreter.effect_output(output);
                ProcessOutput::Running
            }
            EffectHandler::Spawn(handler) => {
                let output = handler.to_output(&input);
                interpreter.effect_output(output);
                let manifest = handler.spawn(&input);
                vm.spawn(&manifest);
                ProcessOutput::Running
            }
            EffectHandler::Defer => ProcessOutput::Performed { input, effect },
            EffectHandler::SendMessage(handler) => {
                let output = handler.to_output(&input);
                interpreter.effect_output(output);
                let SendMessage { to, ty, message } = handler.send_message(&input);
                if let Some(to) = vm.read_dprocesses().get(&to) {
                    to.receive_message(vm, ty, message);
                }
                ProcessOutput::Running
            }
            EffectHandler::ReceiveMessage => {
                let message_type = effect.output;
                // lock mailbox after status is safe.
                if let Some(message) = self
                    .lock_mailbox()
                    .get_mut(&message_type)
                    .and_then(|queue| queue.pop_front())
                {
                    interpreter.effect_output(message);
                    ProcessOutput::Running
                } else {
                    // Don't update status like `*status = new_status`.
                    self.update_status(
                        vm,
                        &mut status,
                        DProcessStatus::WaitingForMessage(message_type),
                    );
                    ProcessOutput::WaitingForMessage
                }
            }
            EffectHandler::FlushMailbox => {
                let message_type = effect.output;
                // lock mailbox after status is safe.
                let messages = self
                    .lock_mailbox()
                    .get_mut(&message_type)
                    .map(|queue| queue.drain(..).collect())
                    .unwrap_or_else(Vec::new);
                interpreter.effect_output(Value::Vector(messages));
                ProcessOutput::Running
            }
            EffectHandler::Subscribe(handler) => {
                let output = handler.to_output(&input);
                interpreter.effect_output(output);
                let ty = handler.subscribe(&input);
                vm.subscribe(self.id.clone(), ty);
                ProcessOutput::Running
            }
            EffectHandler::Publish => {
                let ty = effect.input;
                interpreter.effect_output(Value::Unit);

                // This is required because the publish() may locks them.
                drop(interpreter);
                drop(status);

                vm.publish(ty, input);
                ProcessOutput::Running
            }
            EffectHandler::GetKv(handler) => {
                // read lock KV after status is safe.
                let output = handler.to_output(&input, &self.read_kv());
                interpreter.effect_output(output);
                ProcessOutput::Running
            }
            EffectHandler::UpdateKv(handler) => {
                // lock KV after status is safe.
                let output = handler.update(&input, &mut self.lock_kv());
                interpreter.effect_output(output);
                ProcessOutput::Running
            }
            EffectHandler::GetFlags(handler) => {
                // no need to release or keep the lock, so release them.
                drop(status);

                let dprocess_id = handler.target_dprocess_id(&input);
                let output = match vm.get_dprocess(&dprocess_id) {
                    Some(dprocess) => handler.to_output(&input, Some(&*dprocess.read_flags())),
                    None => handler.to_output(&input, None),
                };
                interpreter.effect_output(output);
                ProcessOutput::Running
            }
            EffectHandler::UpdateFlags(handler) => {
                // no need to release or keep the lock, so release them.
                drop(status);

                let dprocess_id = handler.target_dprocess_id(&input);
                let output = match vm.get_dprocess(&dprocess_id) {
                    Some(dprocess) => {
                        handler.update_flags(&input, Some(&mut *dprocess.lock_flags()))
                    }
                    None => handler.update_flags(&input, None),
                };
                interpreter.effect_output(output);
                ProcessOutput::Running
            }
            EffectHandler::AddTimer(handler) => {
                let output = handler.to_output(&input);
                interpreter.effect_output(output);
                let manifest = handler.add_timer(&input);

                // no need to release or keep the locks, so release them.
                drop(interpreter);
                drop(status);

                // lock timers after status is safe.
                self.lock_timers()
                    // TODO: remove clone()
                    .insert(manifest.name.clone(), Timer::new(manifest));
                ProcessOutput::Running
            }
            EffectHandler::RemoveTimer(handler) => {
                let output = handler.to_output(&input);
                interpreter.effect_output(output);
                let name = handler.remove_timer(&input);
                // lock timers after status is safe.
                self.lock_timers().remove(&name);
                ProcessOutput::Running
            }
            EffectHandler::Monitor(handler) => {
                let output = handler.to_output(&input);
                interpreter.effect_output(output);

                let target = handler.monitor(&input);
                if let Some(target) = vm.get_dprocess(&target) {
                    target.add_monitor(self);
                } else {
                }
                ProcessOutput::Running
            }
            EffectHandler::Demonitor(handler) => {
                let output = handler.to_output(&input);
                interpreter.effect_output(output);

                let target = handler.demonitor(&input);
                if let Some(target) = vm.get_dprocess(&target) {
                    target.remove_monitor(&self.id);
                } else {
                }
                ProcessOutput::Running
            }
            EffectHandler::ProcessInfo(handler) => {
                // Unlock is required because handler may need read locks of them.
                drop(interpreter);
                drop(status);

                let info = DProcessInfo::new(self);
                let output = handler.to_output(&input, info);
                // lock interpreter here is safe because we have dropped the locks.
                self.lock_interpreter().effect_output(output);
                ProcessOutput::Running
            }
            EffectHandler::VmInfo(handler) => {
                let output = handler.to_output(&input, &vm);
                interpreter.effect_output(output);
                ProcessOutput::Running
            }
            EffectHandler::Link(handler) => {
                let output = handler.to_output(&input);
                interpreter.effect_output(output);

                // release the locks before dprocess.link.
                drop(interpreter);
                drop(status);

                let (id1, id2) = handler.link(&input);
                match (vm.get_dprocess(&id1), vm.get_dprocess(&id2)) {
                    (Some(dprocess1), Some(dprocess2)) => {
                        dprocess1.add_link(vm, &dprocess2);
                    }
                    (Some(dprocess1), None) => {
                        dprocess1.link_not_found(vm, id2);
                    }
                    (None, Some(dprocess2)) => {
                        dprocess2.link_not_found(vm, id1);
                    }
                    _ => {}
                }
                ProcessOutput::Running
            }
            EffectHandler::Unlink(handler) => {
                let output = handler.to_output(&input);
                interpreter.effect_output(output);

                // release the locks before dprocess.unlink.
                drop(interpreter);
                drop(status);

                let (id1, id2) = handler.unlink(&input);
                if let Some((dprocess1, dprocess2)) =
                    vm.get_dprocess(&id1).zip(vm.get_dprocess(&id2))
                {
                    dprocess1.remove_link(&dprocess2);
                }
                ProcessOutput::Running
            }
            EffectHandler::Register(handler) => {
                let output = handler.to_output(&input);
                interpreter.effect_output(output);

                let (name, id) = handler.register(&input);
                vm.register(name, id);

                ProcessOutput::Running
            }
            EffectHandler::Unregister(handler) => {
                let output = handler.to_output(&input);
                interpreter.effect_output(output);

                let name = handler.unregister(&input);
                vm.unregister(name);

                ProcessOutput::Running
            }
            EffectHandler::Whereis(handler) => {
                let output = handler.to_output(&input, &vm.read_name_registry());
                interpreter.effect_output(output);
                ProcessOutput::Running
            }
            EffectHandler::Halt(handler) => {
                let output = handler.to_output(&input);
                interpreter.effect_output(output);

                // release locks before dprocess.halt().
                drop(interpreter);
                drop(status);

                let HaltProcess { id, ty, reason } = handler.halt(&input);
                if let Some(dprocess) = vm.get_dprocess(&id) {
                    dprocess.update_status(
                        vm,
                        &mut dprocess.lock_status(),
                        DProcessStatus::Halted {
                            ty: Arc::new(ty),
                            reason: Arc::new(reason),
                        },
                    );
                }
                ProcessOutput::Running
            }
        }
    }
}

#[derive(Debug)]
pub enum ProcessOutput {
    Running,
    WaitingForMessage,
    Performed { input: Value, effect: Effect },
    Returned(Arc<Value>),
    Halted { ty: Type, reason: Value },
    Crashed(Arc<anyhow::Error>),
}
