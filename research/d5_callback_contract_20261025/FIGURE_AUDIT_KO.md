# D5 저장 결과 그림 감사

## 판정

최종 두 PNG를 root와별도 C9 reviewer가 실제로 열어 확인했다. 최종판정은
PASS_SCOPED이며 추가plot 실행은 필요없다. 그림은90mm(3.55in) 폭으로 설계했고
8–10pt 문자가 읽힌다.180mm 확대에서도 clipping/overlap 문제는 관측되지 않았다.
PDF는같은plot에서export했고별도의PDF raster rendering은하지않았다.

## 캡션

**D5_DERIVATIVE_WITNESSES.** A: x=1/4,y=3/4 slice의9개저장된J[0,1] 값과
transpose 해석값J[1,0]을비교한다. 연결선은점의관계를읽는보조이며continuous
solution/tube certificate가아니다. B: 각p=3..9에서7개 t=i/8의time-stencil
오차최대, 총49개stencil이다. 관측오차0은해당dyadic제조다항식에서exact rational
secant/derivative와일치한다는뜻이다. 점선은사전등록64eps기준1.4210854715202004e-14다.
일반callback의finite-difference정확도나모든step size에대한오차0으로넓히지않는다.

**D5_BOUNDARY_WITNESSES.** A: 저장된true-J값의transpose/off-diagonal sign/J+I
세semantic interpretation이원exactderivative와다른행수다. 각각720/729,
729/729,729/729이며compiledRust mutant실행횟수가아니다. B: 실제7개invalidquery와
7개invalidstencil은0개승인,finite full-write J+I callback control한개는1개승인됐다.
이1/1은gate의의도된interface범위와derivative-truth검산의차이를보인다.

## 실제 finding과 최소 교정

| Finding | 실제 이미지에서 본 문제 | 교정과 보존 |
|---|---|---|
| FIG-ANNOTATION | 첫A패널의x/y주석이빨간점선과접촉 | 패널주석을지우고캡션에고정state를옮김 |
| FIG-LABEL | Outside stencil이zero/NaN/sub-ulp invalidstep을충분히표현하지못함 | Invalid query/Invalid stencil로라벨수정 |

원본PNG/PDF와원plot source는 evidence/figure_initial/에보존했다. 저장된scientific
JSONL/REVIEW와gateway/test바이트는바뀌지않았다. plotter만추가1회실행했고그시간과
원본matplotlib cache경고를BUDGET/log에보존했다. cache경고뒤허용된/tmp에생성된
cache로성공했으며과학실패또는과거캠페인재실행으로분류하지않는다.

## 최종 잔여 검사

- 0오차는로그축바닥이나표시floor로만든값이아니다. 실제저장된0을선형축에표시했다.
- transpose미검출9개는t=0,y=3/4의대칭J행이며검증실패를숨기지않았다.
- marker/색/점선으로원값과잘못된해석이구분된다. panel A/B의단위와분모가다르다.
- 원래시각문제는제거됐고data/tolerance/model/유도에는변경이없다.

최종hash와실행identity는 SOURCE_EXECUTION_IDENTITY.json에있다.
