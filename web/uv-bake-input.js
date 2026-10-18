// One actual worker at a time, with captured owner identity and monotonic Rust ID.
export function createUVBakeWorkers(getHandle,makeWorker,notice){
  let active;
  const cancel=id=>{
    if(active?.id!==id)return;
    const old=active;active=undefined;old.worker.terminate();
  };
  const start=(id,project,size,channel)=>{
    if(active)throw new Error('A texture export is already running.');
    const owner=getHandle();
    if(!owner||!Number.isInteger(id)||id<1||id>0xffffffff||![256,512,1024,2048,4096].includes(size)||!['color','roughness','metallic'].includes(channel)||typeof project!=='string'||project.length>128*1024*1024)throw new Error('Invalid texture export request.');
    const worker=makeWorker(),request={id,worker,owner,size};active=request;
    const finish=(success,png,report)=>{
      if(active!==request)return;
      active=undefined;worker.terminate();
      if(getHandle()!==owner)return;
      try { owner.complete_uv_bake(id,success,png,report); }
      catch(error) { notice(`Texture export failed: ${error}`); }
    };
    worker.onmessage=event=>{
      const r=event.data;
      if(r?.id!==id)return;
      if(typeof r.success!=='boolean'||!(r.png instanceof Uint8Array)||r.png.byteLength>size*size*4+size+1024*1024||typeof r.report!=='string'||r.report.length>4096){finish(false,new Uint8Array(),'Invalid or oversized texture worker result.');return;}
      finish(r.success,r.png,r.report);
    };
    worker.onerror=event=>{event.preventDefault();finish(false,new Uint8Array(),String(event.message||'Texture worker failed.').slice(0,1024));};
    worker.onmessageerror=()=>finish(false,new Uint8Array(),'Texture worker result could not be read.');
    try { worker.postMessage({id,project,size,channel}); }
    catch(error) { active=undefined;worker.terminate();throw error; }
  };
  return {start,cancel};
}
