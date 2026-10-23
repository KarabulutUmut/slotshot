// Non-sensitive, in-memory fixture images only. No stored PNG is read or edited.
export function sampleThumbnails(): string[] {
  const drawings = [
    '<rect width="320" height="240" fill="#b9d7e5"/><path d="M0 220 105 40 225 240M95 240 240 65 320 230" fill="#526b83"/><path d="m105 40-24 42 29-11 27 15z" fill="#eff3ef"/>',
    '<rect width="320" height="240" fill="#1e2329"/>' +
      Array.from(
        { length: 9 },
        (_, i) =>
          `<rect x="25" y="${28 + i * 22}" width="${[160, 240, 190][i % 3]}" height="5" fill="${i % 3 ? "#a5b7ba" : "#8bb59d"}"/>`,
      ).join(""),
    '<rect width="320" height="240" fill="#f2f0e9"/><rect x="30" y="28" width="170" height="12" fill="#3c4650"/>' +
      Array.from(
        { length: 7 },
        (_, i) =>
          `<rect x="30" y="${65 + i * 20}" width="${i % 2 ? 220 : 250}" height="4" fill="#9b9e9e"/>`,
      ).join(""),
  ];
  return drawings.map((drawing) =>
    URL.createObjectURL(
      new Blob(
        [
          `<svg xmlns="http://www.w3.org/2000/svg" width="320" height="240">${drawing}</svg>`,
        ],
        { type: "image/svg+xml" },
      ),
    ),
  );
}
