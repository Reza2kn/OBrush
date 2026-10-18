// Display-free Rust baking, isolated from the interactive viewport.
import init, { uv_bake_operation } from './pkg/obrush_app.js';
const ready=init();
self.onmessage=async event=>{
  const {id,project,size,channel}=event.data;
  let output;
  try {
    await ready;
    output=uv_bake_operation(project,size,channel);
    const png=output.take_png(),report=output.report();
    self.postMessage({id,success:true,png,report},[png.buffer]);
  } catch(error) { self.postMessage({id,success:false,png:new Uint8Array(),report:String(error)}); }
  finally { output?.free(); }
};
