/**
 * The fictional family in the demo. Sarah is Jimmy's mother; FirstRing answers
 * her line, and her own phone rings only after she hears a private summary and
 * presses 1. Jimmy is on her trusted list, so his calls skip screening. Jimmy and
 * his wife Maya run their own FirstRing line, and the two lines share warnings.
 */
export interface Person {
  name: string;
  relation: string;
  role: string;
}

export interface Household {
  id: string;
  name: string;
  summary: string;
  people: Person[];
  /** Who hears the private summary and decides with the keypad. */
  ringsFirst: Person;
}

export const FAMILY: Record<string, Household> = {
  sarah: {
    id: 'sarah',
    name: 'Sarah',
    summary: 'Jimmy’s mother. FirstRing answers her line and screens every unfamiliar caller.',
    people: [
      {
        name: 'Sarah',
        relation: 'Jimmy’s mother',
        role: 'FirstRing answers her calls; her phone rings only after she says yes',
      },
      {
        name: 'Jimmy',
        relation: 'Sarah’s son',
        role: 'On her trusted list, so his calls skip screening',
      },
    ],
    ringsFirst: {
      name: 'Sarah',
      relation: 'Jimmy’s mother',
      role: 'Hears a private summary and presses 1 to talk or 2 to decline',
    },
  },
  maya: {
    id: 'maya',
    name: 'Maya and Jimmy',
    summary: 'Jimmy and his wife Maya. Their line shares warnings with Sarah’s.',
    people: [
      { name: 'Maya', relation: 'Jimmy’s wife', role: 'FirstRing answers their home line' },
      { name: 'Jimmy', relation: 'Maya’s husband', role: 'Shares the line with Maya' },
    ],
    ringsFirst: {
      name: 'Maya',
      relation: 'Jimmy’s wife',
      role: 'Hears a private summary and presses 1 to talk or 2 to decline',
    },
  },
};

export function household(id: string | undefined): Household | undefined {
  return id ? FAMILY[id] : undefined;
}

/** The person whose phone rings for a private summary on this household's line. */
export function ringsFirstName(id: string | undefined, fallback: string): string {
  return household(id)?.ringsFirst.name ?? fallback;
}
