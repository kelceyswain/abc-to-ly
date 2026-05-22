#(set-global-staff-size 18)
\paper {
  %output-suffix = "web-inline"
  fonts = # (make-pango-font-tree "EB Garamond" "Noto Sans" "Nimbus Mono PS" (/ (* staff-height pt) 2.5))

  line-width = 140\mm
  indent = 0\mm
  %ragged-right = ##t

  bookTitleMarkup = ##f
  scoreTitleMarkup = ##f
  oddHeaderMarkup = ##f
  evenHeaderMarkup = ##f
  oddFooterMarkup = ##f
  evenFooterMarkup = ##f
  print-page-number = ##f
  tagline = ##f
}