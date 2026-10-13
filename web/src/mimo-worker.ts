import init,{analyze_mimo} from './wasm/blocksim';
import {type MimoPlan} from './mimo';
const ready=init();
self.onmessage=async(event:MessageEvent<{id:number;plan:MimoPlan}>)=>{const {id,plan}=event.data;try{await ready;self.postMessage({id,result:JSON.parse(analyze_mimo(JSON.stringify(plan.model),JSON.stringify(plan.request)))});}catch(error){self.postMessage({id,error:String(error)});}};
