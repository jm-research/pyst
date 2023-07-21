extern crate pyst_parser;

use super::compile;
use super::pyobject::{PyObjectRef, PyResult};
use super::vm::VirtualMachine;

pub fn eval(vm: &mut VirtualMachine, source: &String, scope: PyObjectRef) -> PyResult {
    match compile::compile(vm, source, compile::Mode::Eval) {
        Ok(bytecode) => {
            debug!("Code object: {:?}", bytecode);
            vm.run_code_obj(bytecode, scope)
        }
        Err(msg) => {
            panic!("Parsing went horribly wrong: {}", msg);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::eval;
    use super::VirtualMachine;

    #[test]
    fn test_print_42() {
        let source = String::from("print('Hello world')\n");
        let mut vm = VirtualMachine::new();
        let vars = vm.context().new_scope(None);
        let _result = eval(&mut vm, &source, vars);

        // TODO: check result?
        //assert_eq!(
        //    parse_ast,
        // );
    }
}