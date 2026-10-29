ClearAll[th];
aa={{0,0,0,0,0,0,0},{1/5,0,0,0,0,0,0},{3/40,9/40,0,0,0,0,0},{44/45,-56/15,32/9,0,0,0,0},{19372/6561,-25360/2187,64448/6561,-212/729,0,0,0},{9017/3168,-355/33,46732/5247,49/176,-5103/18656,0,0},{35/384,0,500/1113,125/192,-2187/6784,11/84,0}};
cc={0,1/5,3/10,4/5,8/9,1,1};
bb={35/384,0,500/1113,125/192,-2187/6784,11/84,0};
pp={{1,-8048581381/2820520608,8663915743/2820520608,-12715105075/11282082432},{0,0,0,0},{0,131558114200/32700410799,-68118460800/10900136933,87487479700/32700410799},{0,-1754552775/470086768,14199869525/1410260304,-10690763975/1880347072},{0,127303824393/49829197408,-318862633887/49829197408,701980252875/199316789632},{0,-282668133/205662961,2019193451/616988883,-1453857185/822651844},{0,40617522/29380423,-110615467/29380423,69997945/29380423}};
bt=pp.{th,th^2,th^3,th^4};
res=FullSimplify[{Total[bt]-th,bt.cc-th^2/2,bt.(cc^2)-th^3/3,bt.(aa.cc)-th^3/6,bt.(cc^3)-th^4/4,bt.(cc*(aa.cc))-th^4/8,bt.(aa.(cc^2))-th^4/12,bt.(aa.aa.cc)-th^4/24}];
endpoint=FullSimplify[(bt/.th->1)-bb];
d0=FullSimplify[(D[bt,th]/.th->0)-UnitVector[7,1]];
d1=FullSimplify[(D[bt,th]/.th->1)-UnitVector[7,7]];
fifthMomentResidual=Factor[bt.(cc^4)-th^5/5];
ExportString[<|"continuous_order4_residuals"->res,"endpoint_weight_residuals"->endpoint,"left_derivative_residuals"->d0,"right_derivative_residuals"->d1,"row_sum_residuals"->(aa.ConstantArray[1,7]-cc),"all_order4_and_endpoint_checks_pass"->And[res===ConstantArray[0,8],endpoint===ConstantArray[0,7],d0===ConstantArray[0,7],d1===ConstantArray[0,7]],"fifth_moment_residual_exact"->ToString[fifthMomentResidual,InputForm],"fifth_moment_at_half_exact"->ToString[fifthMomentResidual/.th->1/2,InputForm],"scope"->"New continuous extension only; existing endpoint order-five proof not repeated."|>,"RawJSON"]
