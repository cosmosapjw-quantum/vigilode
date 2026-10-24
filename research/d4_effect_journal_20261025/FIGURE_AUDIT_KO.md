# 저장된 protocol 그림 검토

그림 D4_PROTOCOL_EVIDENCE.png/PDF를 실제로 열어 읽었다. 표본은 raw trace cap1의 ALSA(accept→apply 뒤 오류→동일 batch 재전달 성공→추가 advance)이며 producer=[1,1,1,1], sink=[0,1,2,2], effect=[0,1,1,1], pending attempt=[1,1,1,1]이다. 수용된 효과를 재전달해도 controlled dedup sink의 effect 수는1이며, 별도 ACK 전에는 producer가 진행하지 않는다.

왼쪽 패널은 16,384개 새 protocol case에서 cap0–3별 관측 최대 producer 수가0,1,2,3임을 보인다. 이것은 유한 시퀀스 검산이고 모든 길이에 대한 결과는 THEORY의 귀납증명으로 연결한다.

의도한 인쇄폭은 double-column 약183mm다. 제목/축/범례/주석의 잘림이나 겹침은 관측되지 않았다. 90mm 단일열로 전체2패널을 축소하면 텍스트가 작아지므로 해당사용에서는패널별배치 또는PDF확대를사용한다. Semantic/CRAG PASS_SCOPED: 축은호출/효과개수이며 ODE오차·wall-time비교를표시하지않는다. Mutation3개는 REVIEW의 실제 witness로 식별되며 그림을 일반 외부 sink 보장으로 승격하지 않는다. 코드·과학 결과를 바꿀 residual finding은0.
