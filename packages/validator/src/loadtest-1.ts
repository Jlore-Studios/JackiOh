/**
 * #375's load test for #318: reverse the order of the words in a string. Words split on runs of
 * spaces and join back with one space, so leading, trailing and doubled spaces collapse.
 */
export function reverseWords(text: string): string {
  return text.split(/ +/).filter(Boolean).reverse().join(" ");
}
