use anyhow::{anyhow, Result};
use pyo3::{
    ffi,
    prelude::*,
    types::{PyList, PyTuple},
    FromPyPointer,
};
use std::{borrow::BorrowMut, cell::RefCell, env::args};

thread_local!(static EMB_GLOBAL_DATA: RefCell<usize>  = RefCell::new(0));

extern "C" fn emb_numargs(_s: *mut ffi::PyObject, _args: *mut ffi::PyObject) -> *mut ffi::PyObject {
    Python::with_gil(|py| {
        EMB_GLOBAL_DATA.with(|nargs| {
            *nargs.borrow()
        }).into_py(py).into_ptr()
    })
}

extern "C" fn init_emb() -> *mut ffi::PyObject {
    unsafe {
        let fptr: ffi::PyCFunction = emb_numargs;
        let method_numargs = ffi::PyMethodDef {
            ml_name: "numargs\0".as_ptr() as *const _,
            ml_meth: Some(fptr),
            ml_flags: ffi::METH_VARARGS,
            ml_doc: "Return the number of arguments received by the process\0".as_ptr() as *const _,
        };

        let method_sentinel = ffi::PyMethodDef {
            ml_name: std::ptr::null_mut(),
            ml_meth: None,
            ml_flags: 0,
            ml_doc: std::ptr::null_mut(),
        };

        let mut methods_array: [ffi::PyMethodDef; 2] = [method_numargs, method_sentinel];
        let methods: *mut ffi::PyMethodDef = methods_array.as_mut_ptr();

        let slot_sentinel = ffi::PyModuleDef_Slot {
            slot: 0,
            value: std::ptr::null_mut(),
        };

        let mut slots_array: [ffi::PyModuleDef_Slot; 1] = [slot_sentinel];
        let slots: *mut ffi::PyModuleDef_Slot = slots_array.as_mut_ptr();

        let def: *mut ffi::PyModuleDef = ffi::PyModuleDef {
            m_base: ffi::PyModuleDef_HEAD_INIT,
            m_name: "emb\0".as_ptr() as *const _,
            m_doc: std::ptr::null(),
            m_size: 0,
            m_methods: methods,
            //XXX weird, since the exact same statement seems to work in derive_utils.rs:304
            m_slots: std::ptr::null_mut(),
            // m_slots: slots,
            m_traverse: None,
            m_clear: None,
            m_free: None,
        }.borrow_mut();

        ffi::PyModule_Create(def)
    }
}


fn main() -> Result<()> {
    EMB_GLOBAL_DATA.with(|nargs| {
        *nargs.borrow_mut() = args().len();
    });

    unsafe { ffi::PyImport_AppendInittab("emb\0".as_ptr() as *const _, Some(init_emb)); }
    pyo3::prepare_freethreaded_python();

    let mut argiter = args();
    argiter.next(); // Skip over program name
    let aname = argiter.next();
    let afunc = argiter.next();

    let (pname, pfunc) = aname
        .zip(afunc)
        .ok_or_else(|| anyhow!("Usage: call pythonfile funcname [args]"))?;

    let value = Python::with_gil(|py| -> PyResult<i32> {
        let argtuple = unsafe {
            let tpl = ffi::PyTuple_New(argiter.len().try_into()?);
            argiter.enumerate().for_each(|(i, v)| {
                ffi::PyTuple_SetItem(
                    tpl,
                    i as isize,
                    v.parse::<i32>().unwrap().into_py(py).into_ptr(),
                );
            });
            PyTuple::from_borrowed_ptr(py, tpl)
        };
        // Add current dir to path
        let sys = PyModule::import(py, "sys")?;
        let path: &PyList = sys.getattr("path")?.extract()?;
        path.insert(0, ".")?;

        let module = PyModule::import(py, &pname)?;
        let func = module.getattr(pfunc)?;
        let val: i32 = func.call1(argtuple)?.extract()?;
        Ok(val)
    })?;

    println!("Result of call: {}", value);

    Ok(())
}
