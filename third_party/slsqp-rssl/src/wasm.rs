#[cfg(feature = "wasm")]
use wasm_bindgen::prelude::*;

#[cfg(feature = "wasm")]
#[wasm_bindgen]
pub fn fmin_slsqp_wasm(
    n: usize,
    obj_func: &js_sys::Function,
    x0: Vec<f64>,
    lb: Vec<f64>,
    ub: Vec<f64>,
    eq_cons: Option<js_sys::Array>,
    ineq_cons: Option<js_sys::Array>,
    max_iter: usize,
    acc: f64,
    callback: Option<js_sys::Function>,
    event_observer: Option<js_sys::Function>,
) -> crate::SlsqpResult {
    let func = |x: &[f64]| {
        let x_js = js_sys::Float64Array::from(x);
        obj_func
            .call1(&JsValue::NULL, &x_js)
            .unwrap()
            .as_f64()
            .unwrap()
    };

    let mut bounds = Vec::with_capacity(n);
    for i in 0..n {
        bounds.push((lb[i], ub[i]));
    }

    let mut constraints = Vec::new();

    if let Some(eqs) = eq_cons {
        for f_val in eqs.iter() {
            let f = js_sys::Function::from(f_val);
            constraints.push(crate::Constraint::Eq(Box::new(move |x| {
                let x_js = js_sys::Float64Array::from(x);
                f.call1(&JsValue::NULL, &x_js).unwrap().as_f64().unwrap()
            })));
        }
    }

    if let Some(ineqs) = ineq_cons {
        for f_val in ineqs.iter() {
            let f = js_sys::Function::from(f_val);
            constraints.push(crate::Constraint::Ineq(Box::new(move |x| {
                let x_js = js_sys::Float64Array::from(x);
                f.call1(&JsValue::NULL, &x_js).unwrap().as_f64().unwrap()
            })));
        }
    }

    let observer = WasmObserver {
        callback,
        event_observer,
    };

    crate::fmin_slsqp_observed(func, &x0, &bounds, constraints, max_iter, acc, observer)
}

#[cfg(feature = "wasm")]
struct WasmObserver {
    callback: Option<js_sys::Function>,
    event_observer: Option<js_sys::Function>,
}

#[cfg(feature = "wasm")]
impl crate::SlsqpObserver for WasmObserver {
    fn is_active(&self) -> bool {
        true
    }

    fn on_event(&mut self, event: crate::SlsqpEvent) {
        if let Some(ref eo) = self.event_observer {
            let js_event = event_to_js(&event);
            let _ = eo.call1(&JsValue::NULL, &js_event);
        }

        if let Some(ref cb) = self.callback {
            let crate::SlsqpEvent::Step {
                iter,
                mode,
                x,
                f,
                g,
                c,
                alpha,
                s,
                h,
            } = event;

            let step = crate::OptimizationStep {
                iter,
                mode,
                x: x.to_vec(),
                fun: f,
                grad: g.to_vec(),
                constraints: c.to_vec(),
                l: h.to_vec(),
                alpha,
                s: s.to_vec(),
            };
            let _ = cb.call1(&JsValue::NULL, &JsValue::from(step));
        }
    }
}

#[cfg(feature = "wasm")]
fn event_to_js(event: &crate::SlsqpEvent) -> JsValue {
    let obj = js_sys::Object::new();
    match event {
        crate::SlsqpEvent::Step {
            iter,
            mode,
            x,
            f,
            g,
            c,
            alpha,
            s,
            h,
        } => {
            let _ = js_sys::Reflect::set(&obj, &"type".into(), &"Step".into());
            let _ = js_sys::Reflect::set(&obj, &"iter".into(), &(*iter as f64).into());
            let _ = js_sys::Reflect::set(&obj, &"mode".into(), &(*mode as i32 as f64).into());
            let _ = js_sys::Reflect::set(&obj, &"x".into(), &js_sys::Float64Array::from(*x).into());
            let _ = js_sys::Reflect::set(&obj, &"f".into(), &(*f).into());
            let _ = js_sys::Reflect::set(&obj, &"g".into(), &js_sys::Float64Array::from(*g).into());
            let _ = js_sys::Reflect::set(&obj, &"c".into(), &js_sys::Float64Array::from(*c).into());
            let _ = js_sys::Reflect::set(&obj, &"alpha".into(), &(*alpha).into());
            let _ = js_sys::Reflect::set(&obj, &"s".into(), &js_sys::Float64Array::from(*s).into());
            let _ = js_sys::Reflect::set(&obj, &"h".into(), &js_sys::Float64Array::from(*h).into());
        }
    }
    obj.into()
}
