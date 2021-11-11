use anyhow::{anyhow, Result};
use pyo3::{
    ffi,
    prelude::*,
    types::{PyList, PyTuple},
    FromPyPointer,
};
use std::{cell::RefCell, env::args};

thread_local!(static EMB_GLOBAL_DATA: RefCell<usize>  = RefCell::new(0));

#[pymodule]
fn emb(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(numargs, m)?)?;
    Ok(())
}

#[pyfunction]
/// Return the number of arguments of the application command line
fn numargs() -> PyResult<usize> {
    EMB_GLOBAL_DATA.with(|nargs| {
        Ok(*nargs.borrow())
    })
}


fn main() -> Result<()> {
    // Initialize the numargs variable
    EMB_GLOBAL_DATA.with(|nargs| {
        *nargs.borrow_mut() = args().len();
    });

    unsafe {
        // Make the emb.numargs() function accessible to the embedded Python interpreter.
        // This makes use of the fact that #[pymodule] creates an initialization function with the
        // name `PyInit_<funcname>`.
        let ptr = std::mem::transmute::<*const (), extern "C" fn() -> *mut ffi::PyObject>(PyInit_emb as *const ());
        ffi::PyImport_AppendInittab("emb\0".as_ptr() as *const _, Some(ptr));
    }
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
