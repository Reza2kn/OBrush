// The same Rust geometry engine runs here without a viewport or WebRunner.
import init, { topology_operation } from './pkg/obrush_app.js';
const ready=init();
self.onmessage=async event=>{
  const {id,project,operation}=event.data;
  try {
    await ready;
    const contents=topology_operation(project,operation);
    self.postMessage({id,success:true,contents});
  } catch(error){self.postMessage({id,success:false,contents:String(error)});}
};
