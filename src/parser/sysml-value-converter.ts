import { DefaultValueConverter } from 'langium';
import type { CstNode, GrammarAST, ValueType } from 'langium';

const ESCAPES: Record<string, string> = { b: '\b', f: '\f', n: '\n', r: '\r', t: '\t', v: '\v' };

/**
 * A name in single quotes is the same name without them, its escapes
 * resolved (§8.2.2.1.2): `'Brew unit'` is Brew unit, and `'Engine'` is Engine.
 * That holds in references too, whose text is their names joined by `::`.
 */
export class SysmlValueConverter extends DefaultValueConverter {
    protected override runConverter(rule: GrammarAST.AbstractRule, input: string, cstNode: CstNode): ValueType {
        return rule.name === 'Name' ? unquote(input) : super.runConverter(rule, input, cstNode);
    }
}

function unquote(name: string): string {
    if (name.length < 2 || !name.startsWith("'") || !name.endsWith("'")) {
        return name;
    }
    return name.slice(1, -1).replace(/\\(.)/g, (_, c: string) => ESCAPES[c] ?? c);
}
