import init,{analyze_frequency} from './wasm/blocksim';
import {type FrequencyPlan} from './frequency';
const ready=init();
self.onmessage=async(event:MessageEvent<{id:number,plan:FrequencyPlan}>)=>{
 const {id,plan}=event.data;try{await ready;self.postMessage({id,result:JSON.parse(analyze_frequency(JSON.stringify(plan.model),JSON.stringify(plan.request)))});}catch(error){self.postMessage({id,error:String(error)});}
};
