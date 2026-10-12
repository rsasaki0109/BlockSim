import init, { run_graph } from './wasm/blocksim';
const ready=init();
self.onmessage=async (event:MessageEvent<{json:string,id:number}>)=>{
  try { await ready; const result=JSON.parse(run_graph(event.data.json)); self.postMessage({id:event.data.id,result}); }
  catch(error){self.postMessage({id:event.data.id,error:String(error)});}
};
