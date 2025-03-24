import React from 'react'

export default function Separator(rest = {}) {
  return <div className="separator" {...rest} />
}

export function VerticalSeparator(rest = {}) {
  return <div className="vertical-separator" {...rest} />
}
