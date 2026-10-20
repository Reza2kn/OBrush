// Normal asynchronous URL transport only. Rust captures flags/revision/order and
// validates the whole file; this helper never changes document/camera/pick data.
export function createStartupModelInput(getHandle, canvas, notice, baseURL=window.location.href) {
  const LIMIT=128*1024*1024;
  // Browser exception strings are implementation-defined and can include the
  // literal request URL. Only our own bounded, credential-free messages leave
  // this transport boundary.
  class ModelInputError extends Error {}
  let current=null;
  const cancel=id=>{if(current?.id===id){current.controller.abort();current=null;}};
  const start=(id,rawURL)=>{
    if(current)current.controller.abort();
    const controller=new AbortController();const request={id,controller};current=request;
    const timeout=setTimeout(()=>controller.abort(),30000);
    void (async()=>{
      try {
        const url=new URL(rawURL,baseURL);
        if(!['http:','https:'].includes(url.protocol))throw new ModelInputError('Model URL requires HTTP or HTTPS.');
        const response=await fetch(url,{signal:controller.signal,credentials:'omit',mode:'cors',cache:'no-store'});
        if(response.status!==200)throw new ModelInputError(`Model GET requires HTTP200; returned ${response.status}.`);
        const declared=response.headers.get('content-length');
        if(declared!==null&&Number(declared)>LIMIT)throw new ModelInputError('Model exceeds128MiB.');
        if(!response.body)throw new ModelInputError('Streaming model body unavailable.');
        const reader=response.body.getReader();const chunks=[];let size=0;
        try {
          while(true){const {done,value}=await reader.read();if(done)break;
            if(!(value instanceof Uint8Array)||size+value.byteLength>LIMIT){await reader.cancel();throw new ModelInputError('Model exceeds128MiB.');}
            size+=value.byteLength;chunks.push(value);
          }
        } finally {reader.releaseLock();}
        if(current!==request||controller.signal.aborted)return;
        // At most128MiB input plus128MiB concatenation; chunks are released after
        // completion. Rust performs its independent bound/preflight again.
        const bytes=new Uint8Array(size);let offset=0;for(const chunk of chunks){bytes.set(chunk,offset);offset+=chunk.length;}
        chunks.length=0;
        getHandle().load_startup_model_bytes(id,bytes);canvas.focus();
      } catch(error) {
        if(current===request){const message=controller.signal.aborted?'Model read cancelled or timed out.':error instanceof ModelInputError?error.message:'Model URL read failed. Check URL, CORS and server response.';
          getHandle()?.fail_startup_model(id,message);notice(`Startup model failed: ${message}`);}
      } finally {clearTimeout(timeout);if(current===request)current=null;}
    })();
  };
  return Object.freeze({start,cancel});
}
