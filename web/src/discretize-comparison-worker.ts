import init,{discretize_tustin,discretize_zoh,run_graph_budgeted} from './wasm/blocksim';
import {decodeConversionPlan,conversionModels,conversionReport} from './discretize-comparison';
import type {ConversionResult} from './discretize';
import type {GraphResult} from './graph-model';
const ready=init();
self.onmessage=async(event:MessageEvent<{id:number;json:string}>)=>{
 const {id,json}=event.data;
 try {
  await ready;const plan=decodeConversionPlan(json);
  const conversion=JSON.parse((plan.method==='zoh'?discretize_zoh:discretize_tustin)(JSON.stringify(plan.request))) as ConversionResult;
  const [a,b]=conversionModels(plan,conversion);
  // Independent limits bound the aggregate at 20 million component evaluations;
  // no partial result is displayed after either run fails.
  const continuous=JSON.parse(run_graph_budgeted(JSON.stringify(a),10_000_000)) as GraphResult;
  const discrete=JSON.parse(run_graph_budgeted(JSON.stringify(b),10_000_000)) as GraphResult;
  self.postMessage({id,result:conversionReport(plan,conversion,continuous,discrete)});
 }catch(error){self.postMessage({id,error:String(error)});}
};
