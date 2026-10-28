# 이번 source-polynomial 단위의 문헌 대조

SciSpace에 multiaffine vertex bounds와보존법칙을소거한mass-action Jacobian/Hessian에관한완전한질문을보내10개논문메타데이터를받았다. 이검색의다수는평형/보존법칙에관한것으로,이번전체성분범위정리의직접근거로채택하지않았다. Exa Search에서서로맞닿은vertex/chemical-Jacobian범위를8결과로조사했고같은논문2개URL을중복으로식별했다. 실제관련원전은저자PDF로읽었다. 검색결과수는독립논문전체검토수와구분한다.

채택한원전: Franco Blanchini, Patrizio Colaneri, Giulia Giordano, Irene Zorzan (2022), *Vertex results for the robust analysis of uncertain biochemical systems*, Journal of Mathematical Biology85:35. DOI https://doi.org/10.1007/s00285-022-01799-z ; 저자본문 https://giuliagiordano.dii.unitn.it/docs/papers/Journal/J055_2022_JoMB.pdf . 원전은uncertainbiochemicalsystems의multiaffine구조와hyperrectanglevertex방법을연결한다.

이번증명은더직접적이다. 실제BASS의J성분이z전체와ratevector에각각affine이므로productsimplex/ratebox에서연속선형최적화의꼭짓점성질을두번적용한다. 원전의모든minor가multiaffine인조건이나안정성·역행렬의vertex정리를자동으로가져오지않는다. 이클래식원리를새일반정리나strongno-go로주장하지않는다. 이번기여는고정source의전자feedback·neutral소거·실제좌표·physicalmetricremainder와실행가능한exactwitness를연결하는것이다.

Wolfram context는맞는특수source결과를찾지못했다. 관련없는Jacobi/특수함수결과는사용하지않는다. 승인후별도로고정할한번의symbolic polynomial identity 검산은그실제출력으로만보고한다. Context호출은CAS평가완료를뜻하지않는다.

원시응답: SCISPACE.json, EXA.json, EXA_PRIMARY_VERTEX_PAPER.json, WOLFRAM_CONTEXT.json. 웹본문ref turn21view0은현재채팅인용용이다. 원문에서파생한내용은위범위뿐이며source-specific증명은THEORY_KO.md에직접작성한다.
