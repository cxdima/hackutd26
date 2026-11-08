import type { Observation, RequestedObject } from './schemas';

/** Plain-language vocabulary shared by the landing page, dashboard and trace. */

export const OBJECT_LABEL: Record<RequestedObject, string> = {
  authentication_secret: 'a sign-in or verification code',
  appointment_reference: 'an appointment reference',
  payment: 'an ordinary payment',
  payment_diversion: 'money sent an unusual way, such as gift cards or a wire',
  remote_access: 'remote access to a device',
  routine_information: 'routine information',
  unknown: 'something that was not clear yet',
};

export const PROTECTED_OBJECTS: readonly RequestedObject[] = [
  'authentication_secret',
  'payment_diversion',
  'remote_access',
];

export function isProtectedRequest(observation: Observation): boolean {
  return (
    observation.speech_act === 'request' &&
    observation.direction === 'caller_requests_from_sarah' &&
    PROTECTED_OBJECTS.includes(observation.requested_object)
  );
}

/** One sentence describing a model assessment, e.g. "The caller asked Sarah for a sign-in code." */
export function describeObservation(observation: Observation, name: string): string {
  const object = OBJECT_LABEL[observation.requested_object];
  switch (observation.speech_act) {
    case 'request':
      if (observation.direction === 'caller_requests_from_sarah')
        return `The caller asked ${name} for ${object}.`;
      if (observation.direction === 'caller_supplies') return `The caller offered ${object}.`;
      return `The caller asked about ${object}.`;
    case 'statement':
      return observation.direction === 'caller_supplies'
        ? `The caller shared ${object}.`
        : `The caller mentioned ${object}.`;
    case 'negation':
      return `The caller mentioned ${object} without asking for it (for example, a warning not to share it).`;
    case 'quotation':
      return `The caller quoted or reported ${object}; it was not a request.`;
    default:
      return 'What the caller wanted was not clear yet.';
  }
}

export const TOOL_LABEL: Record<string, string> = {
  record_assessment: 'Assess the caller’s request',
  ask_caller: 'Ask the caller a follow-up question',
  offer_handoff: 'Offer a private handoff',
  take_message: 'Save a private message',
  end_call: 'End the call politely',
  lookup_tactics: 'Look up similar shared tactics',
  lookup_trusted_contact: 'Look up an enrolled contact',
  start_trusted_callback: 'Request an independent contact',
  publish_tactic: 'Share a tactic capsule',
};

export function toolLabel(name: string): string {
  return TOOL_LABEL[name] ?? name.replaceAll('_', ' ');
}

export const TACTIC_LABEL: Record<string, string> = {
  authentication_code_extraction: 'Asking for a sign-in code',
  payment_diversion: 'Asking for money sent an unusual way',
  remote_access: 'Asking for remote access to a device',
};
export const PRETEXT_LABEL: Record<string, string> = {
  bank_account: 'Claims to be from the bank',
  tech_support: 'Claims to be tech support',
  delivery: 'Claims to be a delivery service',
  government: 'Claims to be a government office',
  family_emergency: 'Claims a family emergency',
  clinic: 'Claims to be a clinic',
  unknown: 'Pretext not identified',
};
export const PRESSURE_LABEL: Record<string, string> = {
  none: 'No pressure observed',
  urgency: 'Rushes you to act now',
  secrecy: 'Asks you to keep it secret',
  threat: 'Threatens consequences',
  authority: 'Leans on authority',
};
export const ACTION_LABEL: Record<string, string> = {
  share_authentication_secret: 'Share a sign-in or verification code',
  send_payment: 'Send a payment',
  grant_remote_access: 'Give remote access',
  independent_contact: 'Hang up and call the organization at a number you already know',
};

export function humanize(value: string, table: Record<string, string>): string {
  return table[value] ?? value.replaceAll('_', ' ').replace(/^\w/, (c) => c.toUpperCase());
}

export function plural(count: number, one: string, many = `${one}s`): string {
  return `${count} ${count === 1 ? one : many}`;
}
