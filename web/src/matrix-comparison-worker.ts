import init,{discretize_matrix,run_graph_budgeted} from './wasm/blocksim';
import {decodeMatrixPlan,matrixComparisonModels,matrixComparisonReport} from './matrix-comparison';
import type {MatrixResult} from './matrix-discretize';
import type {GraphResult} from './graph-model';
const ready=init();
self.onmessage=async(event:MessageEvent<{id:number;json:string}>)=>{const {id,json}=event.data;try{
 await ready;const plan=decodeMatrixPlan(json),conversion=JSON.parse(discretize_matrix(JSON.stringify(plan.request))) as MatrixResult;
 const [a,b]=matrixComparisonModels(plan,conversion),continuous=JSON.parse(run_graph_budgeted(JSON.stringify(a),10_000_000)) as GraphResult,discrete=JSON.parse(run_graph_budgeted(JSON.stringify(b),10_000_000)) as GraphResult;
 self.postMessage({id,result:matrixComparisonReport(plan,conversion,continuous,discrete)});
}catch(error){self.postMessage({id,error:String(error)});}};
