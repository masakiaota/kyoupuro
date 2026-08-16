const RELATIVE_SCORE_SCALE = 1_000_000_000;
const RELATIVE_SCORE_SCALE_BIGINT = 1_000_000_000n;

function asNonNegativeScore(value) {
  const score = Number(value);
  return Number.isFinite(score) && score >= 0 ? score : null;
}

function relativeScoreForCase(score, maxScore) {
  if (score == null || maxScore == null || maxScore <= 0) {
    return 0;
  }

  // eval.py records integer absolute scores. Use integer arithmetic here so a value exactly at
  // the x.5 boundary follows the contest's round rule even when the scores are large.
  if (Number.isSafeInteger(score) && Number.isSafeInteger(maxScore)) {
    const scoreBig = BigInt(score);
    const maxScoreBig = BigInt(maxScore);
    return Number(
      (2n * RELATIVE_SCORE_SCALE_BIGINT * scoreBig + maxScoreBig) / (2n * maxScoreBig),
    );
  }

  return Math.round((RELATIVE_SCORE_SCALE * score) / maxScore);
}

// This is a local reproduction of the contest's relative-score calculation. The available
// evaluation runs, rather than all contest participants, provide each case's maximum score.
export function withRelativeScores(runs, caseNames) {
  const normalizedCaseNames = Array.isArray(caseNames) ? caseNames : [];
  const maxScoreByCase = new Map();

  for (const run of runs) {
    for (const caseName of normalizedCaseNames) {
      const score = asNonNegativeScore(run.caseScores?.[caseName]);
      if (score == null) {
        continue;
      }
      const previous = maxScoreByCase.get(caseName);
      if (previous == null || score > previous) {
        maxScoreByCase.set(caseName, score);
      }
    }
  }

  return runs.map((run) => {
    let relativeScore = 0;
    for (const caseName of normalizedCaseNames) {
      relativeScore += relativeScoreForCase(
        asNonNegativeScore(run.caseScores?.[caseName]) ?? 0,
        maxScoreByCase.get(caseName),
      );
    }
    return { ...run, relativeScore };
  });
}
