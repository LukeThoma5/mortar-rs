export function makeAction<TApiAction, TActionType extends string>(
  apiAction: TApiAction,
  actionType: TActionType
): TApiAction & {
  toString(): TActionType;
} {
  (apiAction as any).toString = () => actionType;
  return apiAction as TApiAction & {
    toString(): TActionType;
  };
}
