import init,{discretize_matrix,run_graph_budgeted} from './wasm/blocksim';
import {decodeMatrixPlan,matrixComparisonModels,matrixComparisonReport} from './matrix-comparison';
import type {MatrixResult} from './matrix-discretize';
import type {GraphResult} from './graph-model';
import {verifyMatrixResult} from './matrix-comparison-file';
import {decodeMatrixFile} from './matrix-comparison-session';
const ready=init();
self.onmessage=async(event:MessageEvent<{id:number;json:string;restore?:boolean}>)=>{const {id,json,restore}=event.data;try{
 await ready;const stored=restore?decodeMatrixFile(json).report:null,plan=stored?stored.plan:decodeMatrixPlan(json),conversion=JSON.parse(discretize_matrix(JSON.stringify(plan.request))) as MatrixResult;
 const [a,b]=matrixComparisonModels(plan,conversion),continuous=JSON.parse(run_graph_budgeted(JSON.stringify(a),10_000_000)) as GraphResult,discrete=JSON.parse(run_graph_budgeted(JSON.stringify(b),10_000_000)) as GraphResult;
 const result=matrixComparisonReport(plan,conversion,continuous,discrete);if(stored)verifyMatrixResult(stored,result);self.postMessage({id,result});
}catch(error){self.postMessage({id,error:String(error)});}};
