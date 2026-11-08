import copy
import json
import pathlib
import unittest
import xgrammar as xgr
from firstring_compact_tools import bound_whitespace, compact_structural_tag

SCHEMA = {
    'type':'object', 'additionalProperties':False,
    'properties':{
        'summary':{'type':'string'},
        'understanding':{'type':'object','additionalProperties':False,
            'properties':{'supplied_facts':{'type':'array','items':{'type':'string'}},
                'version':{'type':'integer','enum':[1]},
                'purpose':{'type':'string','enum':['routine message','warning']},
                'quote':{'type':'string'},'ready':{'type':'boolean'}},
            'required':['supplied_facts','version','purpose','quote','ready']}},
    'required':['summary','understanding']}
VALUE={'summary':'Never  share a code — café 🦊','understanding':{'supplied_facts':[],
    'version':1,'purpose':'warning','quote':'Never  share your code — café 🦊','ready':True}}

def make_tag(schema):
    return xgr.get_model_structural_tag(model='qwen_3_coder',
        tools=[{'type':'function','function':{'name':'record_assessment','parameters':schema}}],
        tool_choice={'type':'function','function':{'name':'record_assessment'}},reasoning=False)

def xml(value):
    return '<tool_call>\n<function=record_assessment>\n'+'\n'.join(
        '<parameter='+key+'>'+(v if isinstance(v,str) else json.dumps(v,ensure_ascii=False,separators=(',',':')))
        +'</parameter>' for key,v in value.items())+'\n</function>\n</tool_call>'

def compiled(tag):
    info=xgr.TokenizerInfo([bytes([i]) for i in range(256)]+[b'<eos>'],stop_token_ids=[256])
    return xgr.GrammarCompiler(info).compile_structural_tag(tag)

def accepts(grammar,text):
    matcher=xgr.GrammarMatcher(grammar)
    return matcher.accept_string(text) and matcher.is_completed()

def minimal(schema):
    if 'const' in schema:return schema['const']
    if 'enum' in schema:return schema['enum'][0]
    kind=schema.get('type')
    if isinstance(kind,list):
        if 'null' in kind:return None
        kind=kind[0]
    if kind=='object':return {k:minimal(v) for k,v in schema['properties'].items() if k in schema.get('required',[])}
    if kind=='array':return [minimal(schema['items']) for _ in range(schema.get('minItems',0))]
    if kind=='string':return 'x'*max(1,schema.get('minLength',0))
    if kind=='integer':return 1
    if kind=='boolean':return True
    if kind=='null':return None
    raise ValueError('Unsupported authored fixture schema')

class CompactGrammarTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.original=compiled(make_tag(SCHEMA));cls.compact=compiled(compact_structural_tag(make_tag(SCHEMA)))

    def test_rejects_whitespace_loop_accepted_by_original(self):
        text=xml(VALUE).replace('"supplied_facts":[]','"supplied_facts":[]'+' '*1000)
        self.assertTrue(accepts(self.original,text))
        self.assertFalse(accepts(self.compact,text))

    def test_preserves_literal_spaces_negation_unicode_and_native_xml(self):
        self.assertTrue(accepts(self.original,xml(VALUE)))
        self.assertTrue(accepts(self.compact,xml(VALUE)))
        extra=copy.deepcopy(VALUE);extra['understanding']['quote']='Never'+(' '*16)+'share your code — café 🦊'
        self.assertTrue(accepts(self.compact,xml(extra)))

    def test_preserves_required_enum_type_and_unknown_key_rejection(self):
        variants=[]
        for mutate in [lambda v:v['understanding'].pop('version'),
                       lambda v:v['understanding'].update(version=2),
                       lambda v:v['understanding'].update(purpose='unlisted'),
                       lambda v:v['understanding'].update(ready='true'),
                       lambda v:v['understanding'].update(extra='not allowed')]:
            value=copy.deepcopy(VALUE);mutate(value);variants.append(value)
        for value in variants:
            with self.subTest(value=value):
                self.assertFalse(accepts(self.original,xml(value)))
                self.assertFalse(accepts(self.compact,xml(value)))

    def test_lexical_rewrite_preserves_quoted_atoms_and_character_classes(self):
        grammar=r'root ::= ("[ \\n\\t]*" [ \n\t]* [^\"\\])'+'\n'
        changed,count=bound_whitespace(grammar)
        self.assertEqual(count,1)
        self.assertIn(r'"[ \\n\\t]*"',changed)
        self.assertIn(r'[^\"\\]',changed)
        self.assertIn(r'[ \n\t]{0,2}',changed)

    def test_current_assessment_schema_keeps_validation_and_literal_evidence(self):
        schema=json.loads(pathlib.Path(__file__).with_name('current-assessment-schema.json').read_text())
        before=compiled(make_tag(schema));after=compiled(compact_structural_tag(make_tag(schema)))
        value=minimal(schema);value['evidence_refs'][0]['quote']='Never  share your code — café 🦊'
        self.assertTrue(accepts(before,xml(value)));self.assertTrue(accepts(after,xml(value)))
        loop=xml(value).replace('"supplied_facts":[]','"supplied_facts":[]'+' '*1000)
        self.assertTrue(accepts(before,loop));self.assertFalse(accepts(after,loop))
        invalid=copy.deepcopy(value);invalid['understanding'].pop('version')
        self.assertFalse(accepts(before,xml(invalid)));self.assertFalse(accepts(after,xml(invalid)))
        invalid=copy.deepcopy(value);invalid['observation']['direction']='unlisted'
        self.assertFalse(accepts(before,xml(invalid)));self.assertFalse(accepts(after,xml(invalid)))

    def test_plugin_preserves_upstream_native_extraction(self):
        from transformers import AutoTokenizer
        from vllm.entrypoints.openai.chat_completion.protocol import ChatCompletionRequest
        from vllm.tool_parsers.qwen3_engine_tool_parser import Qwen3EngineToolParser
        from firstring_compact_tools import FirstRingCompactQwen3Parser
        snapshot=pathlib.Path('/root/.cache/huggingface/hub/models--nvidia--NVIDIA-Nemotron-3.5-Lightning-30B-A3B-NVFP4/snapshots/bee7596271d1495f6992ae224aefde4410e816b8')
        tokenizer=AutoTokenizer.from_pretrained(snapshot,local_files_only=True)
        request=ChatCompletionRequest(model='synthetic',messages=[{'role':'user','content':'synthetic'}],
            tools=[{'type':'function','function':{'name':'record_assessment','parameters':SCHEMA}}],
            tool_choice={'type':'function','function':{'name':'record_assessment'}})
        baseline=Qwen3EngineToolParser(tokenizer,tools=request.tools)
        candidate=FirstRingCompactQwen3Parser(tokenizer,tools=request.tools)
        original=baseline.extract_tool_calls(xml(VALUE),request)
        changed=candidate.extract_tool_calls(xml(VALUE),request)
        self.assertEqual(original.content,changed.content)
        self.assertEqual([x.function.model_dump() for x in original.tool_calls],[x.function.model_dump() for x in changed.tool_calls])
        self.assertTrue(changed.tools_called)
        self.assertEqual(json.loads(changed.tool_calls[0].function.arguments),VALUE)
        self.assertIs(FirstRingCompactQwen3Parser.extract_tool_calls,Qwen3EngineToolParser.extract_tool_calls)
        self.assertTrue(accepts(compiled(candidate.get_structural_tag(request)),xml(VALUE)))

if __name__=='__main__':unittest.main()
