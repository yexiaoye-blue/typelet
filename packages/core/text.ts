export const characters = (text: string) => Array.from(text.normalize('NFC'));
export const limitPracticeInput = (value: string, target: string) => characters(value).slice(0, characters(target).length).join('');

