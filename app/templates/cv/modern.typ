// Modern CV Template - Typst
// Clean, contemporary design with subtle accents

#let page-margin = 1in

// Theme variables - overridden by data injection
#let accent-color = rgb("#0ea5e9")
#let text-color = rgb("#1e293b")
#let muted-color = rgb("#64748b")
#let bg-color = rgb("#f8fafc")
#let dark-text = rgb("#0f172a")
#let rule-color = accent-color
#let card-bg = rgb("#ffffff")
#let card-border = rgb("#e2e8f0")

#set page(
  margin: page-margin,
  paper: "a4",
  fill: bg-color,
)

#set text(
  font: "Inter",
  size: 10pt,
  fill: text-color,
  line-height: 1.5,
)

#show link: it => {
  set text(fill: accent-color)
  it
}

// Header with name and contacts
#let header(name, contacts) = {
  align(center + horizon, [
    text(size: 28pt, weight: "bold", fill: dark-text, name)
    #v(4pt)
    text(size: 10pt, fill: muted-color, contacts.join("  •  "))
  ])
  #v(4pt)
  #rule(width: 100%, thickness: 2pt, stroke: rule-color)
  #v(12pt)
}

// Section title with rule
#let section-title(title) = {
  text(size: 12pt, weight: "bold", fill: dark-text, upper(title))
  #h(4pt)
  #rule(width: 100%, thickness: 1pt, stroke: rule-color)
  #v(8pt)
}

// Experience/education entry header
#let entry-header(title, org, date, location) = {
  table(
    columns: (1fr, auto),
    [
      [text(weight: "semibold", title), text(size: 9pt, fill: muted-color, date)],
      [text(fill: accent-color, org), text(size: 9pt, fill: muted-color, location)],
    ]
  )
  #v(4pt)
}

// Description text
#let entry-desc(desc) = {
  text(desc)
  #v(6pt)
}

// Bullet list
#let bullet-list(items) = {
  #for item in items {
    - #item
  }
}

// Skill tags
#let skill-tags(skills) = {
  #for skill in skills {
    box(
      fill: accent-color.lighten(90%),
      stroke: accent-color,
      radius: 4pt,
      inset: (x: 6pt, y: 2pt),
      text(size: 8.5pt, weight: "medium", fill: accent-color, skill),
    )
    #h(4pt)
  }
}

// Language entries
#let language-entry(name, proficiency) = {
  table(
    columns: (1fr, auto),
    [text(weight: "medium", name), text(size: 9pt, fill: muted-color, proficiency)],
  )
  #v(3pt)
}

// Project entry
#let project-entry(name, desc, techs, links) = {
  text(weight: "semibold", name)
  #v(2pt)
  text(size: 9.5pt, desc)
  #v(4pt)
  text(size: 9pt, fill: muted-color, techs.join("  •  "))
  #if links.len() > 0 {
    #v(4pt)
    #for link in links {
      #link
      #h(8pt)
    }
  }
  #v(8pt)
}

// Certification entry
#let cert-entry(name, issuer, date, expiry, url, cred-id) = {
  entry-header(name, issuer, date, "")
  #if expiry != "" {
    text(size: 9pt, fill: muted-color, "Expires: " + expiry)
    #v(3pt)
  }
  #if url != "" {
    #link(url)[Verify]
    #if cred-id != "" { #h(8pt) text(size: 9pt, fill: muted-color, "ID: " + cred-id) }
  } #else if cred-id != "" {
    text(size: 9pt, fill: muted-color, "ID: " + cred-id)
  }
  #v(8pt)
}

// ============================================================
// MAIN RENDER FUNCTION - Called with injected data
// ============================================================

#let render-cv(data) = {
  // Extract personal info
  let personal = data.personal
  let contacts = (
    personal.email,
    personal.phone,
    personal.location,
    personal.linkedin,
    personal.github,
    personal.portfolio
  ).filter(x => x != "")
  
  // Header
  header(personal.full_name, contacts)
  
  // Summary
  #if personal.summary != "" {
    section-title("Summary")
    text(personal.summary)
    #v(12pt)
  }
  
  // Experience
  #if data.experience.len() > 0 {
    section-title("Experience")
    #for exp in data.experience {
      entry-header(exp.title, exp.company, exp.date, exp.location)
      entry-desc(exp.description)
      #if exp.skills.len() > 0 {
        text(size: 9pt, weight: "medium", fill: muted-color, "Skills: " + exp.skills.join(", "))
        #v(4pt)
      }
      #v(8pt)
    }
  }
  
  // Education
  #if data.education.len() > 0 {
    section-title("Education")
    #for edu in data.education {
      entry-header(edu.degree, edu.institution, edu.date, edu.location)
      #if edu.field != "" {
        text(size: 9.5pt, fill: muted-color, edu.field)
        #v(2pt)
      }
      #if edu.grade != "" {
        text(size: 9pt, fill: muted-color, "Grade: " + edu.grade)
        #v(2pt)
      }
      #if edu.description != "" {
        #v(4pt)
        entry-desc(edu.description)
      }
      #v(8pt)
    }
  }
  
  // Skills
  #if data.skills.len() > 0 {
    section-title("Skills")
    #let grouped = data.skills.group(x => x.category ?? "General")
    #for (cat, skills) in grouped {
      #if cat != "General" {
        text(size: 10pt, weight: "semibold", fill: dark-text, cat)
        #v(3pt)
      }
      skill-tags(skills.map(x => x.name + x.level.map(l => " (" + l + ")").or("")))
      #v(6pt)
    }
  }
  
  // Languages
  #if data.languages.len() > 0 {
    section-title("Languages")
    #for lang in data.languages {
      language-entry(lang.name, lang.proficiency)
    }
    #v(4pt)
  }
  
  // Projects
  #if data.projects.len() > 0 {
    section-title("Projects")
    #for proj in data.projects {
      project-entry(proj.name, proj.description, proj.technologies, proj.links)
    }
  }
  
  // Certifications
  #if data.certifications.len() > 0 {
    section-title("Certifications")
    #for cert in data.certifications {
      cert-entry(cert.name, cert.issuer, cert.date, cert.expiry, cert.url, cert.credential_id)
    }
  }
}

// The data structure expected:
// data = (
//   personal: (
//     full_name: "",
//     email: "",
//     phone: "",
//     location: "",
//     linkedin: "",
//     github: "",
//     portfolio: "",
//     summary: "",
//   ),
//   experience: (
//     (title: "", company: "", location: "", date: "", description: "", skills: ()), ...
//   ),
//   education: (
//     (degree: "", institution: "", location: "", date: "", field: "", grade: "", description: ""), ...
//   ),
//   skills: (
//     (name: "", category: "", level: ""), ...
//   ),
//   languages: (
//     (name: "", proficiency: ""), ...
//   ),
//   projects: (
//     (name: "", description: "", technologies: (), links: ()), ...
//   ),
//   certifications: (
//     (name: "", issuer: "", date: "", expiry: "", url: "", credential_id: ""), ...
//   ),
// )