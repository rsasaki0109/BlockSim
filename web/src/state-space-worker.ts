import init,{analyze_state_space,analyze_discrete_state_space} from './wasm/blocksim';
import {type StateSpacePlan} from './state-space';
const ready=init();
self.onmessage=async(event:MessageEvent<{id:number;plan:StateSpacePlan}>)=>{const {id,plan}=event.data;try{await ready;self.postMessage({id,result:JSON.parse((plan.domain==='discrete'?analyze_discrete_state_space:analyze_state_space)(JSON.stringify(plan.model),JSON.stringify(plan.request)))});}catch(error){self.postMessage({id,error:String(error)});}};
