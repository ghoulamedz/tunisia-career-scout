// Minimal CV Template - Typst
// Minimalist design focusing on content

#let page-margin = 0.9in

// Theme variables - overridden by data injection
#let accent-color = rgb("#374151")
#let text-color = rgb("#111827")
#let muted-color = rgb("#9ca3af")
#let bg-color = rgb("#ffffff")
#let line-color = rgb("#e5e7eb")
#let card-bg = rgb("#f9fafb")
#let card-border = rgb("#e5e7eb")

#set page(
  margin: page-margin,
  paper: "a4",
  fill: bg-color,
)

#set text(
  font: "Inter",
  size: 10pt,
  fill: text-color,
  line-height: 1.7,
)

// Header with name and contacts
#let header(name, contacts) = {
  align(left, [
    text(size: 22pt, weight: "bold", fill: text-color, name)
    #v(4pt)
    text(size: 9pt, fill: muted-color, contacts.join("  ·  "))
  ])
  #v(2pt)
  #rule(width: 60%, thickness: 1pt, stroke: accent-color)
  #v(16pt)
}

// Section title with rule
#let section-title(title) = {
  text(size: 11pt, weight: "semibold", fill: text-color, upper(title))
  #v(2pt)
  #rule(width: 40%, thickness: 1pt, stroke: accent-color)
  #v(10pt)
}

// Entry for experience/education
#let entry(title, org, date, location, desc) = {
  table(
    columns: (auto, 1fr),
    [
      [text(size: 9pt, fill: muted-color, date), text(weight: "bold", title)],
      [text(size: 9pt, fill: muted-color, location), text(fill: accent-color, org)],
    ]
  )
  #v(3pt)
  text(size: 9.5pt, desc)
  #v(10pt)
}

// Simple bullet list
#let simple-list(items) = {
  #for item in items {
    - #text(size: 9.5pt, item)
  }
}

// Skill line
#let skill-line(skills) = {
  text(size: 9.5pt, skills.join("  ·  "))
  #v(6pt)
}

// Language entry
#let language-entry(name, proficiency) = {
  text(size: 9.5pt, name + ": " + proficiency)
  #v(3pt)
}

// Project entry
#let project-entry(name, desc, techs, links) = {
  text(weight: "semibold", name)
  #v(2pt)
  text(size: 9.5pt, desc)
  #v(3pt)
  text(size: 9pt, fill: muted-color, techs.join("  ·  "))
  #if links.len() > 0 {
    #v(3pt)
    #for link in links {
      #link
      #h(10pt)
    }
  }
  #v(8pt)
}

// Certification entry
#let cert-entry(name, issuer, date, expiry, url, cred-id) = {
  text(weight: "semibold", name)
  #v(1pt)
  text(size: 9pt, fill: muted-color, issuer + "  ·  " + date)
  #if expiry != "" {
    #h(8pt)
    text(size: 9pt, fill: muted-color, "Expires: " + expiry)
  }
  #if url != "" {
    #h(8pt)
    #link(url)[Verify]
  }
  #if cred-id != "" {
    #h(8pt)
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
    #v(16pt)
  }
  
  // Experience
  #if data.experience.len() > 0 {
    section-title("Experience")
    #for exp in data.experience {
      entry(exp.title, exp.company, exp.date, exp.location, exp.description)
      #if exp.skills.len() > 0 {
        text(size: 9pt, fill: muted-color, exp.skills.join("  ·  "))
        #v(6pt)
      }
    }
  }
  
  // Education
  #if data.education.len() > 0 {
    section-title("Education")
    #for edu in data.education {
      entry(edu.degree, edu.institution, edu.date, edu.location, "")
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
        text(size: 9.5pt, edu.description)
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
        text(size: 10pt, weight: "semibold", fill: text-color, cat)
        #v(3pt)
      }
      skill-line(skills.map(x => x.name + x.level.map(l => " (" + l + ")").or("")))
    }
  }
  
  // Languages
  #if data.languages.len() > 0 {
    section-title("Languages")
    #for lang in data.languages {
      language-entry(lang.name, lang.proficiency)
    }
    #v(6pt)
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