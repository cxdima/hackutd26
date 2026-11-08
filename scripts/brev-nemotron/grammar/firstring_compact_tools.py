"""Pinned qwen3_coder parser: bound formatting whitespace, keep native extraction.

Only the generated EBNF atom [ \\n\\t]* outside quoted strings/character
classes is narrowed to zero through two characters. JSON string contents,
XML wrappers, required properties, enums and types remain the upstream rules.
No response text is rewritten or repaired.
"""
import importlib.metadata
import xgrammar as xgr
from xgrammar.structural_tag import GrammarFormat

WHITESPACE_ATOM = r'[ \n\t]*'
BOUNDED_ATOM = r'[ \n\t]{0,2}'


def bound_whitespace(ebnf):
    output, count, index = [], 0, 0
    while index < len(ebnf):
        if ebnf[index] == '"':
            start = index
            index += 1
            while index < len(ebnf):
                if ebnf[index] == '\\':
                    index += 2
                elif ebnf[index] == '"':
                    index += 1
                    break
                else:
                    index += 1
            else:
                raise ValueError('Unterminated generated EBNF literal')
            output.append(ebnf[start:index])
        elif ebnf.startswith(WHITESPACE_ATOM, index):
            output.append(BOUNDED_ATOM)
            index += len(WHITESPACE_ATOM)
            count += 1
        elif ebnf[index] == '[':
            start = index
            index += 1
            while index < len(ebnf):
                if ebnf[index] == '\\':
                    index += 2
                elif ebnf[index] == ']':
                    index += 1
                    break
                else:
                    index += 1
            else:
                raise ValueError('Unterminated generated EBNF character class')
            output.append(ebnf[start:index])
        else:
            output.append(ebnf[index])
            index += 1
    return ''.join(output), count


def compact_structural_tag(tag):
    if tag is None:
        return None
    if importlib.metadata.version('xgrammar') != '0.2.3':
        raise ValueError('This generated-grammar adapter is pinned to xgrammar0.2.3')
    ebnf = str(xgr.Grammar.from_structural_tag(tag))
    bounded, count = bound_whitespace(ebnf)
    if not count:
        raise ValueError('Expected generated whitespace atoms are absent; refused')
    # Round-trip through supported APIs; unsupported grammar fails before decoding.
    verified = xgr.Grammar.from_ebnf(bounded)
    return xgr.StructuralTag(format=GrammarFormat(grammar=str(verified)))


def register_parser():
    if importlib.metadata.version('vllm') != '0.27.1':
        raise ValueError('This parser adapter is pinned to vLLM0.27.1')
    from vllm.tool_parsers import ToolParserManager
    from vllm.tool_parsers.qwen3_engine_tool_parser import Qwen3EngineToolParser

    class FirstRingCompactQwen3Parser(Qwen3EngineToolParser):
        def get_structural_tag(self, request, *, reasoning=False):
            return compact_structural_tag(super().get_structural_tag(request, reasoning=reasoning))

    ToolParserManager.register_module(name='firstring_qwen3_compact',
                                     module=FirstRingCompactQwen3Parser, force=False)
    return FirstRingCompactQwen3Parser


# vLLM's documented --tool-parser-plugin imports this module at startup.
FirstRingCompactQwen3Parser = register_parser()
