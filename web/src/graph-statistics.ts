import type { GraphModel, GraphResult, SolverStatistics } from './graph-model';
/** Old RK4 result files remain byte-compatible; derive their exact event counts. */
export function solverStatistics(model:GraphModel,result:GraphResult):SolverStatistics{
 if(result.statistics)return result.statistics;
 const events=new Set(result.times),end=result.times.at(-1)!;
 const visit=(g:GraphModel)=>{for(const b of g.blocks){if(b.params.kind==='step'){const t=Number(b.params.step_time);if(t>0&&t<end)events.add(t);}else if(b.params.kind==='subsystem')visit(b.params.model as GraphModel);}};visit(model);
 const times=[...events].sort((a,b)=>a-b);let min=Infinity,max=0;for(let i=1;i<times.length;i++){const h=times[i]-times[i-1];min=Math.min(min,h);max=Math.max(max,h);}
 return {accepted_steps:times.length-1,rejected_steps:0,rate_evaluations:4*(times.length-1),output_evaluations:result.times.length,min_step:min,max_step:max,max_error_ratio:0};
}
