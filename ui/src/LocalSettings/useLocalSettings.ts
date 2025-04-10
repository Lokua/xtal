import { useContext } from 'react'
import Context, { ContextProps } from './Context'

export default function useLocalSettings(): ContextProps {
  return useContext(Context)
}
