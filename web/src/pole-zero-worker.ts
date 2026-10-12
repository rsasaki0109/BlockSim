import init,{analyze_pole_zero} from './wasm/blocksim';
import {type PoleZeroPlan} from './pole-zero';
const ready=init();
self.onmessage=async(event:MessageEvent<{id:number;plan:PoleZeroPlan}>)=>{const {id,plan}=event.data;try{await ready;self.postMessage({id,result:JSON.parse(analyze_pole_zero(JSON.stringify(plan.model),JSON.stringify(plan.request)))});}catch(error){self.postMessage({id,error:String(error)});}};
