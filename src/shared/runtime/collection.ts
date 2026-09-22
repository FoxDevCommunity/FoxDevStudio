/**
 * A numbered list of objects, the way Visual FoxPro hands one back.
 *
 * `oProject.Files(1)`, `oAdapter.Tables(1)`, `_VFP.Forms(1)`, `oTable.Fields(1)`: they are all
 * the same thing, so they are all this. `Count` says how many there are, and reading one by
 * number gives it - which reaches the object as a call, because that is how a program writes it.
 */

import type { HostCollection, HostObject } from './oleObjects';
import type { VmValue } from './values';

export class Collection implements HostCollection {
  readonly className = 'Collection';

  /**
   * The keys the items were added under, in the same order. A collection made from a list of
   * objects has none, and `GetKey` then answers with what the reference says: nothing.
   */
  constructor(
    private readonly items: readonly HostObject[],
    private readonly keys: readonly string[] = [],
  ) {}

  /**
   * `GetKey(eIndex)`: the key an item was added under when given its number, and the number
   * when given a key. An index that is not there, or an item added without a key, answers with
   * an empty string.
   */
  getKey(index: VmValue): VmValue {
    if (typeof index === 'number') return this.keys[index - 1] ?? '';
    if (typeof index !== 'string') return '';
    const at = this.keys.findIndex((k) => k.toLowerCase() === index.toLowerCase());
    return at < 0 ? '' : at + 1;
  }

  member(name: string): 'prop' | 'method' | 'none' {
    return name.toUpperCase() === 'COUNT' ? 'prop' : 'method';
  }

  get(name: string): VmValue | HostObject | undefined {
    return name.toUpperCase() === 'COUNT' ? this.items.length : undefined;
  }

  set(): void {
    // nothing of a collection is set: what it holds is decided by whatever holds it
  }

  call(name: string, args: VmValue[]): VmValue | HostObject | undefined {
    // `Item(n)` is the long way of writing what `(n)` says on its own
    if (name.toUpperCase() === 'COUNT') return this.items.length;
    if (name.toUpperCase() === 'GETKEY') return this.getKey(args[0] ?? 0);
    return this.at(Number(args[0] ?? 0));
  }

  at(index: number): HostObject | undefined {
    return this.items[index - 1];
  }
}
