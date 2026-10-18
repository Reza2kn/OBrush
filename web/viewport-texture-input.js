// One reading file and one latest waiting selection. Request ownership lives
// in Rust; this bridge never assigns geometry, tool or document state.
export function createViewportTextureInput(template, getHandle, canvas, report, methods={
  current:'uv_texture_request_is_current',cancel:'cancel_uv_texture_request',load:'load_uv_texture_png_for_request',label:'Texture PNG'
}) {
  let input = template;
  let reading = false;
  let queued = null;
  async function drain() {
    if (reading) return;
    reading = true;
    try {
      while (queued) {
        const job = queued;
        queued = null;
        const owner = getHandle();
        if (!owner || !owner[methods.current](job.id)) continue;
        let submitted = false;
        try {
          if (job.file.size > 16 * 1024 * 1024) throw Error(`${methods.label} exceeds the 16 MiB limit.`);
          const bytes = new Uint8Array(await job.file.arrayBuffer());
          const current = getHandle();
          if (current !== owner || !current[methods.current](job.id)) continue;
          submitted = true;
          current[methods.load](job.id, job.file.name, bytes);
          canvas.focus();
        } catch (error) {
          if (submitted || owner[methods.current](job.id)) {
            owner[methods.cancel](job.id);
            report(`Texture import failed; previous asset kept: ${error}`);
          }
        }
      }
    } finally { reading = false; }
  }
  return id => {
    // A fresh chooser element binds every change/cancel to its own request.
    // Late events from a detached older element cannot borrow a newer ID.
    const fresh = input.cloneNode(false);
    input.replaceWith(fresh);
    input = fresh;
    fresh.addEventListener('cancel', () => getHandle()?.[methods.cancel](id));
    fresh.addEventListener('change', () => {
      const file = fresh.files?.[0];
      if (!file) { getHandle()?.[methods.cancel](id); return; }
      queued = { id, file };
      void drain();
    });
    fresh.value = '';
    fresh.click();
  };
}
