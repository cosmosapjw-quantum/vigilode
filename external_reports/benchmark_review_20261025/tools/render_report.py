"""Render the Korean Markdown report to a portable PDF with embedded font.

Usage: python3 tools/render_report.py --font /path/to/NanumGothic-Regular.ttf
Requires reportlab, matplotlib, Pillow. Never starts a solver or downloads files.
"""
from pathlib import Path
import argparse
import hashlib
import html
import json
import re
import tempfile
from PIL import Image as PILImage
from reportlab.pdfbase import pdfmetrics
from reportlab.pdfbase.ttfonts import TTFont
from reportlab.lib import colors
from reportlab.lib.enums import TA_LEFT
from reportlab.lib.pagesizes import A4
from reportlab.lib.styles import ParagraphStyle
from reportlab.lib.units import mm
from reportlab.platypus import (SimpleDocTemplate, Paragraph, Spacer, Image,
    Table, TableStyle, KeepTogether, CondPageBreak)

ROOT = Path(__file__).resolve().parents[1]
MATH_CHARACTERS = set('λαδμΔθ−⁰¹²³⁴⁵⁶⁷⁸⁹₀₁₂₃ⱼ')


def markup(s):
    s = html.escape(s)
    s = re.sub(r'\*\*(.*?)\*\*', r'<b>\1</b>', s)
    s = re.sub(r'(?<!\*)\*([^*]+)\*(?!\*)', r'<i>\1</i>', s)
    s = re.sub(r'`([^`]+)`', r'<font color="#334b61">\1</font>', s)
    s = s.replace('p̂', '<font name="MathText">p̂</font>')
    for c in MATH_CHARACTERS:
        s=s.replace(c,'<font name="MathText">'+c+'</font>')
    return s


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--font', type=Path, required=True)
    args = parser.parse_args()
    font_path = args.font.resolve()
    pdfmetrics.registerFont(TTFont('Korean', str(font_path)))
    pdfmetrics.registerFontFamily('Korean', normal='Korean', bold='Korean', italic='Korean', boldItalic='Korean')
    pdfmetrics.registerFont(TTFont('MathText', '/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf'))
    pdfmetrics.registerFontFamily('MathText', normal='MathText', bold='MathText', italic='MathText', boldItalic='MathText')
    base = dict(fontName='Korean', fontSize=10.2, leading=15.2, wordWrap='CJK', alignment=TA_LEFT,
                allowWidows=0, allowOrphans=0)
    body = ParagraphStyle('body', **base, spaceAfter=8)
    cap = ParagraphStyle('cap', **{**base,'fontSize':8.5,'leading':12}, textColor=colors.HexColor('#384a5c'), spaceAfter=12)
    title = ParagraphStyle('title', **{**base,'fontSize':22,'leading':32}, spaceAfter=12, textColor=colors.HexColor('#15344f'))
    h2 = ParagraphStyle('h2', **{**base,'fontSize':14,'leading':21}, spaceBefore=16, spaceAfter=9, keepWithNext=True, textColor=colors.HexColor('#15344f'))
    h3 = ParagraphStyle('h3', **{**base,'fontSize':11.3,'leading':18}, spaceBefore=10, spaceAfter=6, keepWithNext=True)
    tab = ParagraphStyle('tab', **{**base,'fontSize':8.1,'leading':12})
    small = ParagraphStyle('small', **{**base,'fontSize':7.4,'leading':10.7})
    width = A4[0]-32*mm
    doc = SimpleDocTemplate(str(ROOT/'VigilODE_Benchmark_Report_KO.pdf'), pagesize=A4,
        leftMargin=16*mm,rightMargin=16*mm,topMargin=17*mm,bottomMargin=17*mm,
        title='VigilODE의 정확도, 계산 비용과 적용 범위',author='VigilODE 연구 자료 독립 재분석',
        subject='Published benchmark evidence and numerical analysis, 2026-10-25')
    source=(ROOT/'REPORT_KO.md').read_text()
    # Full-height two-panel figures form consecutive plates after the discussion.
    # This keeps prose continuous and the 178-mm figures at readable print size.
    sections=re.split(r'(?=^## )',source,flags=re.M)
    plates=[]
    for j,section in enumerate(sections):
        pattern=r'!\[[^\n]+\]\(figures/conditional_[^)]+\)\n\n\*\*그림[^\n]+\n'
        matches=re.findall(pattern,section)
        if matches:
            section=re.sub(pattern,'',section)
            sections[j]=section.rstrip()+'\n\n'
            plates.extend(matches)
    insert=next(j for j,s in enumerate(sections) if s.startswith('## 부록 A.'))
    sections.insert(insert, '\n\n'+'\n'.join(plates)+'\n\n')
    lines=''.join(sections).splitlines()
    story=[]; i=0
    with tempfile.TemporaryDirectory() as tmp:
        while i<len(lines):
            line=lines[i].strip()
            if not line: i+=1;continue
            if line.startswith('# '):story.append(Paragraph(markup(line[2:]),title));i+=1;continue
            if line.startswith('## '):
                story.append(CondPageBreak(38*mm));story.append(Paragraph(markup(line[3:]),h2));i+=1;continue
            if line.startswith('### '):story.append(Paragraph(markup(line[4:]),h3));i+=1;continue
            if line.startswith('!['):
                m=re.match(r'!\[([^]]*)\]\(([^)]+)\)',line);path=ROOT/m.group(2)
                with PILImage.open(path) as im: w,h=im.size
                shownw=width; shownh=shownw*h/w
                if shownh>225*mm:shownh=225*mm;shownw=shownh*w/h
                parts=[Image(str(path),width=shownw,height=shownh),Spacer(1,3*mm)]
                i+=1
                while i<len(lines) and not lines[i].strip():i+=1
                if i<len(lines) and lines[i].startswith('**그림'):
                    parts.append(Paragraph(markup(lines[i]),cap));i+=1
                story.append(KeepTogether(parts));continue
            if line=='\\[':
                eq=[];i+=1
                while lines[i].strip()!='\\]':eq.append(lines[i]);i+=1
                i+=1
                import matplotlib
                matplotlib.use('Agg')
                from matplotlib.mathtext import math_to_image
                out=Path(tmp)/('eq'+str(i)+'.png')
                math_to_image('$'+''.join(eq)+'$',out,dpi=220,format='png')
                with PILImage.open(out) as im:w,h=im.size
                scale=min(1.1, width/(w*72/220))
                story += [Spacer(1,3*mm),Image(str(out),width=w*72/220*scale,height=h*72/220*scale),Spacer(1,5*mm)]
                continue
            if line.startswith('|'):
                rows=[]
                while i<len(lines) and lines[i].strip().startswith('|'):
                    row=[s.strip() for s in lines[i].strip().strip('|').split('|')]
                    if not all(re.fullmatch(r'[: -]+',s) for s in row):rows.append(row)
                    i+=1
                n=len(rows[0]); widths=([.27,.40,.33] if n==3 else [.37,.21,.21,.21] if n==4 else [1/n]*n)
                # Source-version table requires a wider wrapped hash column.
                if rows[0][0]=='자료':widths=[.23,.49,.28]
                cellstyle=small if rows[0][0]=='자료' else tab
                t=Table([[Paragraph(markup(s),cellstyle) for s in row] for row in rows],
                        colWidths=[width*x for x in widths],repeatRows=1,hAlign='LEFT')
                padding=3 if rows[0][0]=='계산 경로' else 6
                t.setStyle(TableStyle([('BACKGROUND',(0,0),(-1,0),colors.HexColor('#e6edf3')),
                    ('VALIGN',(0,0),(-1,-1),'TOP'),('LINEBELOW',(0,0),(-1,0),.7,colors.HexColor('#68859b')),
                    ('LINEBELOW',(0,1),(-1,-1),.25,colors.HexColor('#d4dde5')),
                    ('TOPPADDING',(0,0),(-1,-1),padding),('BOTTOMPADDING',(0,0),(-1,-1),padding),
                    ('LEFTPADDING',(0,0),(-1,-1),6),('RIGHTPADDING',(0,0),(-1,-1),6)]))
                story += [t,Spacer(1,4*mm)];continue
            para=[line];i+=1
            while i<len(lines) and lines[i].strip() and not lines[i].startswith(('#','|','![','\\[')):
                para.append(lines[i].strip());i+=1
            story.append(Paragraph(markup(' '.join(para)),body))
        def page(c,d):
            c.saveState();c.setStrokeColor(colors.HexColor('#c7d2dc'));c.line(16*mm,14*mm,A4[0]-16*mm,14*mm)
            c.setFont('Korean',7.5);c.setFillColor(colors.HexColor('#536879'))
            c.drawString(16*mm,9.5*mm,'VigilODE | 독립 벤치마킹 분석 | 2026-10-25')
            c.drawRightString(A4[0]-16*mm,9.5*mm,str(d.page));c.restoreState()
        doc.build(story,onFirstPage=page,onLaterPages=page)
    record={'font':'NanumGothic-Regular.ttf','font_sha256':hashlib.sha256(font_path.read_bytes()).hexdigest(),
            'font_source':'https://github.com/google/fonts/tree/main/ofl/nanumgothic',
            'source_sha256':hashlib.sha256((ROOT/'REPORT_KO.md').read_bytes()).hexdigest(),
            'pdf_sha256':hashlib.sha256((ROOT/'VigilODE_Benchmark_Report_KO.pdf').read_bytes()).hexdigest()}
    (ROOT/'analysis/RENDER_IDENTITY.json').write_text(json.dumps(record,indent=2,ensure_ascii=False)+'\n')
    print(json.dumps(record))


if __name__=='__main__':main()
