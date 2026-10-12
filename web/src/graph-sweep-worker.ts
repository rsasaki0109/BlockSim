import init,{validate_graph,run_graph_budgeted} from './wasm/blocksim';
import {validateSweepPlan,sweepRow,type SweepPlan} from './graph-sweep';
const ready=init();
self.onmessage=async(event:MessageEvent<{id:number,plan:SweepPlan}>)=>{
 const {id}=event.data;try{
  await ready;const plan=validateSweepPlan(event.data.plan,validate_graph);let evaluations=0;const work=(g:SweepPlan['model']):number=>g.blocks.reduce((n,b)=>n+(b.params.kind==='subsystem'?work(b.params.model as SweepPlan['model']):b.width),0),width=work(plan.model);
  for(let index=0;index<plan.values.length;index++){
   const row=evaluations>=20_000_000?{value:plan.values[index],status:'error' as const,error:'掃引全体の評価回数上限または実行エラーのため未実行です'}:sweepRow(plan,plan.values[index],validate_graph,json=>JSON.parse(run_graph_budgeted(json,20_000_000-evaluations)));
   if(row.statistics)evaluations+=(row.statistics.rate_evaluations+row.statistics.output_evaluations)*width;
   // Failed runs do not expose partial counters; stop rather than undercount them.
   if(row.stage==='execution')evaluations=20_000_000;
   self.postMessage({id,index,row});
  }
  self.postMessage({id,done:true});
 }catch(error){self.postMessage({id,error:String(error)});}
};
