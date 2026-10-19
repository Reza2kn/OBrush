# C275 precise-pointer path: no product change

The prospective C275 gate was frozen in
`SCULPTGL_BRUSH_LOCK_C275_PREREG.md` before input-path inspection. Its
hypothesis was incorrect. The browser's trusted DOM `pointermove` events
already held the rounded coordinates observed by OBrush in C274. The normal
web bridge in `web/index.html` forwards `event.clientX/Y` as JavaScript
numbers to `set_source_paint_input`. `RawPaintInput::receive` stores them in
`Option<[f64; 2]>`; `RawPaintInput::position` returns those doubles, and
`source_paint_host::capture` places them in `SourceGeometryInput.screen`
without passing the coordinate through `egui::Pos2`. The separate egui
pointer remains necessary for UI hit routing and fallback on native input.

The C274 CDP request asked for root held X `813.6924775457785` and held Y
`364.30751877074897`, but the trusted DOM observer recorded
`813.6925048828125` and `364.3075256347656`. Those are precisely the
values of `Math.fround` on the requested coordinates. No Rust `f64`
propagation edit can recover the bits lost before JS receives the event. We
made no C275 product or harness change, did not rerun the frozen C275 trace,
and retain C274 as a failed admission. An integer-coordinate normal-UI
gesture, if pursued, requires a distinct preregistered gate and cannot erase
this fractional-coordinate limitation.
