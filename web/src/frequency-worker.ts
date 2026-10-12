import init,{analyze_frequency,analyze_discrete_frequency} from './wasm/blocksim';
import {type FrequencyPlan} from './frequency';
const ready=init();
self.onmessage=async(event:MessageEvent<{id:number,plan:FrequencyPlan}>)=>{
 const {id,plan}=event.data;try{await ready;self.postMessage({id,result:JSON.parse((plan.domain==='discrete'?analyze_discrete_frequency:analyze_frequency)(JSON.stringify(plan.model),JSON.stringify(plan.request)))});}catch(error){self.postMessage({id,error:String(error)});}
};
